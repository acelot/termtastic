use meshtastic::{api::ConnectedStreamApi, protobufs::FromRadio};
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::{NestedSubsystem, SubsystemBuilder, SubsystemHandle};
use tracing_unwrap::ResultExt;

use crate::meshtastic::{
    RadioService, connect_via_ble, connect_via_serial, connect_via_tcp,
    types::{MeshtasticCommand, MeshtasticEvent},
};

pub struct MeshtasticService {
    command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
    command_rx: mpsc::UnboundedReceiver<MeshtasticCommand>,
    event_tx: broadcast::Sender<MeshtasticEvent>,
    stream_api: Option<ConnectedStreamApi>,
    radio_subsys: Option<NestedSubsystem>,
}

impl MeshtasticService {
    pub fn new() -> (
        Self,
        mpsc::UnboundedSender<MeshtasticCommand>,
        broadcast::Receiver<MeshtasticEvent>,
    ) {
        let (command_tx, command_rx) = mpsc::unbounded_channel::<MeshtasticCommand>();
        let (event_tx, event_rx) = broadcast::channel::<MeshtasticEvent>(100);

        (
            Self {
                command_tx: command_tx.clone(),
                command_rx,
                event_tx,
                stream_api: None,
                radio_subsys: None,
            },
            command_tx.clone(),
            event_rx,
        )
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Some(cmd) = self.command_rx.recv() => self.handle_command(cmd, subsys).await,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: MeshtasticCommand, subsys: &mut SubsystemHandle) {
        match cmd {
            MeshtasticCommand::ConnectViaTcp(hostaddr) => {
                match connect_via_tcp(hostaddr).await {
                    Ok((radio_rx, stream_api)) => {
                        self.handle_connection(radio_rx, stream_api, subsys);

                        self.event_tx
                            .send(MeshtasticEvent::Connected)
                            .unwrap_or_log();
                    }
                    Err(e) => {
                        tracing::error!("can't connect via TCP: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))
                            .unwrap_or_log();
                    }
                };
            }
            MeshtasticCommand::ConnectViaBle(address) => {
                match connect_via_ble(address).await {
                    Ok((radio_rx, stream_api)) => {
                        self.handle_connection(radio_rx, stream_api, subsys);

                        self.event_tx
                            .send(MeshtasticEvent::Connected)
                            .unwrap_or_log();
                    }
                    Err(e) => {
                        tracing::error!("can't connect via BLE: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))
                            .unwrap_or_log();
                    }
                };
            }
            MeshtasticCommand::ConnectViaSerial(address) => {
                match connect_via_serial(address).await {
                    Ok((radio_rx, stream_api)) => {
                        self.handle_connection(radio_rx, stream_api, subsys);

                        self.event_tx
                            .send(MeshtasticEvent::Connected)
                            .unwrap_or_log();
                    }
                    Err(e) => {
                        tracing::error!("can't connect via serial: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))
                            .unwrap_or_log();
                    }
                };
            }
            MeshtasticCommand::Disconnect => {
                if let Some(subsys) = self.radio_subsys.take() {
                    subsys.initiate_shutdown();
                    subsys.join().await.unwrap_or_log();
                }

                if let Some(stream_api) = self.stream_api.take() {
                    stream_api.disconnect().await.ok_or_log();
                }

                self.event_tx
                    .send(MeshtasticEvent::Disconnected)
                    .unwrap_or_log();
            }
            _ => tracing::debug!("unhandled command {:?}", cmd),
        };
    }

    fn handle_connection(
        &mut self,
        radio_rx: mpsc::UnboundedReceiver<FromRadio>,
        stream_api: ConnectedStreamApi,
        subsys: &mut SubsystemHandle,
    ) {
        self.stream_api = Some(stream_api);

        let command_tx = self.command_tx.clone();
        let event_tx = self.event_tx.clone();

        self.radio_subsys = Some(subsys.start(SubsystemBuilder::new(
            "RadioService",
            async |nester_subsys: &mut SubsystemHandle| {
                RadioService::new(command_tx, event_tx)
                    .run(radio_rx, nester_subsys)
                    .await
            },
        )));
    }
}
