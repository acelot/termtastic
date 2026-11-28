use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

use jiff::Timestamp;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::{State, StateAction};
use crate::{APP_NAME, APP_VERSION};

const CHECK_INTERVAL_SECS: i64 = 3600;
const CONFIG_WAIT_SECS: u64 = 10;
const HTTP_TIMEOUT_SECS: u64 = 15;
const UPDATE_CHECK_FILE: &str = "update_check.json";
const RELEASES_URL: &str = "https://api.github.com/repos/acelot/termtastic/releases/latest";

static PARSED_APP_VERSION: LazyLock<Version> = LazyLock::new(|| {
    Version::parse(APP_VERSION.replacen('v', "", 1).as_str())
        .inspect_err(|e| tracing::error!("can't parse APP_VERSION: {}", e))
        .unwrap_or(Version::new(0, 0, 0))
});

#[derive(Debug, Serialize, Deserialize)]
struct UpdateCheckRecord {
    timestamp: Timestamp,
    version: Option<String>,
}

impl Default for UpdateCheckRecord {
    fn default() -> Self {
        Self {
            timestamp: Timestamp::MIN,
            version: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

pub struct UpdateService {
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    data_dir: PathBuf,
    http_client: reqwest::Client,
}

impl UpdateService {
    pub fn new(
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        data_dir: PathBuf,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .expect("HTTP client should be created");

        Self {
            state_rx,
            state_action_tx,
            stored_action_rx,
            data_dir,
            http_client,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut last_check = self.load_last_check();

        let _ = self
            .state_action_tx
            .send(StateAction::UpdateCheckTimestampSet(last_check.timestamp));

        if let Some(version) = last_check.version.as_ref()
            && is_newer(version)
        {
            let _ = self
                .state_action_tx
                .send(StateAction::UpdateAvailableVersionSet(Some(version.clone())));
        }

        let config_deadline = tokio::time::Instant::now() + Duration::from_secs(CONFIG_WAIT_SECS);

        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => match event {
                    Ok(StateAction::AppConfigApply(_)) | Err(broadcast::error::RecvError::Lagged(_)) => break,
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                },
                _ = tokio::time::sleep_until(config_deadline) => {
                    tracing::warn!("app config was not applied in time, using the current state");
                    break;
                },
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    return Ok(());
                }
            }
        }

        let mut delay = {
            let elapsed = Timestamp::now().as_second() - last_check.timestamp.as_second();
            let remaining = (CHECK_INTERVAL_SECS - elapsed).clamp(0, CHECK_INTERVAL_SECS);

            Duration::from_secs(remaining as u64)
        };

        loop {
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }

            if self.state_rx.borrow().disable_update_check {
                tracing::debug!("update check is disabled");
            } else {
                self.check_for_update(&mut last_check).await;
            }

            delay = Duration::from_secs(CHECK_INTERVAL_SECS as u64);
        }

        Ok(())
    }

    async fn check_for_update(&self, last_check: &mut UpdateCheckRecord) {
        let now = Timestamp::now();

        last_check.timestamp = now;

        if let Err(e) = self.save_last_check(last_check) {
            tracing::error!("failed to save update check file: {:#}", e);
        }

        let _ = self.state_action_tx.send(StateAction::UpdateCheckTimestampSet(now));

        match self.fetch_latest_version().await {
            Ok(latest_version) => {
                last_check.version = Some(latest_version.clone());

                if let Err(e) = self.save_last_check(last_check) {
                    tracing::error!("failed to save update check file: {:#}", e);
                }

                if is_newer(&latest_version) {
                    tracing::info!("new version available: {} (current: {})", latest_version, APP_VERSION);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::UpdateAvailableVersionSet(Some(latest_version)));
                } else {
                    tracing::debug!("no new version available (current: {})", APP_VERSION);
                }
            }
            Err(e) => {
                tracing::error!("update check failed: {:#}", e);
            }
        }
    }

    async fn fetch_latest_version(&self) -> anyhow::Result<String> {
        tracing::debug!("HTTP request: GET {}", RELEASES_URL);

        let response = self
            .http_client
            .get(RELEASES_URL)
            .header(reqwest::header::USER_AGENT, format!("{}/{}", APP_NAME, APP_VERSION))
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await?;

        tracing::debug!("HTTP response: GET {} -> {}", RELEASES_URL, response.status());

        let release: GitHubRelease = response.error_for_status()?.json().await?;

        Ok(release.tag_name.trim().to_owned())
    }

    fn check_file_path(&self) -> PathBuf {
        self.data_dir.join(UPDATE_CHECK_FILE)
    }

    fn load_last_check(&self) -> UpdateCheckRecord {
        let path = self.check_file_path();

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return UpdateCheckRecord::default(),
            Err(e) => {
                tracing::warn!("failed to read update check file {}: {}", path.display(), e);

                return UpdateCheckRecord::default();
            }
        };

        match serde_json::from_str::<UpdateCheckRecord>(&content) {
            Ok(record) => record,
            Err(e) => {
                tracing::warn!("failed to parse update check file {}: {}", path.display(), e);

                UpdateCheckRecord::default()
            }
        }
    }

    fn save_last_check(&self, record: &UpdateCheckRecord) -> anyhow::Result<()> {
        std::fs::write(self.check_file_path(), serde_json::to_vec(&record)?)?;

        Ok(())
    }
}

fn is_newer(version: &String) -> bool {
    if let Some(parsed_version) = Version::parse(version.replacen('v', "", 1).as_str())
        .inspect_err(|e| tracing::error!("can't parse version {}: {}", version, e))
        .ok()
    {
        return PARSED_APP_VERSION.lt(&parsed_version);
    }

    false
}
