use std::time::{Duration, Instant};

use futures::stream::{self, StreamExt};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::time;
use tokio_graceful_shutdown::SubsystemHandle;

use crate::types::{AppEvent, ConnectionState, Device, Toast};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
};

const CONNECTION_CHECK_INTERVAL_MILLIS: u64 = 250;
const RECONNECTION_BACKOFF_BASE_MILLIS: u64 = 1000;
const RECONNECTION_BACKOFF_MAX_MILLIS: u64 = 30_000;

pub struct ConnectionService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl ConnectionService {
    pub fn new(
        app_event_tx: broadcast::Sender<AppEvent>,
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            app_event_tx,
            app_event_rx,
            state_rx,
            state_action_tx,
            meshtastic_command_tx,
            meshtastic_event_rx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut connection_check_interval =
            time::interval(Duration::from_millis(CONNECTION_CHECK_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event).await?,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = self.state_rx.changed() => self.handle_state_change()?,
                _ = connection_check_interval.tick() => self.check_connection()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_app_event(&self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::InitializationRequested => {
                self.app_event_tx
                    .send(AppEvent::DeviceRediscoverRequested)?;
            }
            AppEvent::DeviceSelected(hardware) => {
                self.state_action_tx
                    .send(StateAction::DeviceActiveSet(hardware))?;
            }
            AppEvent::DisconnectionRequested => {
                self.meshtastic_command_tx
                    .send(CommandToMeshtastic::Disconnect)?;
            }
            AppEvent::DeviceRediscoverRequested => {
                self.state_action_tx
                    .send(StateAction::Toast(Toast::normal("discovering...")))?;

                match discover_devices().await {
                    Ok(devices) => {
                        let devices_count = devices.len();

                        self.state_action_tx
                            .send(StateAction::DiscoveredDevicesSet(devices))?;

                        self.state_action_tx
                            .send(StateAction::Toast(Toast::normal(format!(
                                "devices discovered: {}",
                                devices_count
                            ))))?;
                    }
                    Err(e) => {
                        tracing::error!("device discovering failed: {}", e);

                        self.state_action_tx
                            .send(StateAction::Toast(Toast::error("discovering failed")))?;
                    }
                };
            }
            AppEvent::TcpDeviceSubmitted(mut hostaddr) => {
                if !hostaddr.has_port() {
                    hostaddr = hostaddr.with_port(4403);
                }

                self.state_action_tx
                    .send(StateAction::DevicesAddTcp(hostaddr))?;
            }
            AppEvent::TcpDeviceRemoved(hostaddr) => {
                self.state_action_tx
                    .send(StateAction::DevicesRemoveTcp(hostaddr))?;
            }
            _ => {}
        }

        Ok(())
    }

    #[allow(unreachable_patterns)]
    fn handle_meshtastic_event(&self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            MeshtasticEvent::Connected => {
                tracing::info!("successfully connected");

                self.state_action_tx.send(StateAction::ConnectionSuccess)?;

                self.state_action_tx
                    .send(StateAction::Toast(Toast::success("connected")))?;
            }
            MeshtasticEvent::ConnectionError(e) => {
                self.state_action_tx.send(StateAction::ConnectionFail(e))?;
            }
            MeshtasticEvent::Disconnected => {
                tracing::info!("disconnected");

                self.state_action_tx.send(StateAction::ConnectionStop)?;

                self.state_action_tx
                    .send(StateAction::Toast(Toast::normal("disconnected")))?;
            }
            MeshtasticEvent::IncomingPacket(_) => {
                self.state_action_tx.send(StateAction::RxTrigger)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_state_change(&self) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        if let Some(device) = &state.active_device
            && state.connection_state == ConnectionState::NotConnected
        {
            self.connect(device)?;
        }

        Ok(())
    }

    fn check_connection(&self) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        if let Some(device) = &state.active_device
            && let ConnectionState::ProblemDetected { since, .. } = state.connection_state
        {
            let backoff_duration = Duration::from_millis(
                (RECONNECTION_BACKOFF_BASE_MILLIS * 2_u64.pow(state.connection_attempt as u32))
                    .min(RECONNECTION_BACKOFF_MAX_MILLIS),
            );

            let time_left = (since + backoff_duration).duration_since(Instant::now());

            self.state_action_tx
                .send(StateAction::ReconnectionBackoffSet(time_left))?;

            if time_left.is_zero() {
                self.connect(device)?;
            }
        }

        Ok(())
    }

    fn connect(&self, device: &Device) -> anyhow::Result<()> {
        self.state_action_tx.send(StateAction::ConnectionStart)?;

        match device {
            Device::Tcp(hostaddr) => self
                .meshtastic_command_tx
                .send(CommandToMeshtastic::ConnectViaTcp(hostaddr.clone()))?,
            Device::Ble { address, .. } => self
                .meshtastic_command_tx
                .send(CommandToMeshtastic::ConnectViaBle(address.to_owned()))?,
            Device::Serial(address) => self
                .meshtastic_command_tx
                .send(CommandToMeshtastic::ConnectViaSerial(address.to_owned()))?,
        };

        Ok(())
    }
}

async fn discover_devices() -> anyhow::Result<Vec<Device>> {
    let mut devices: Vec<Device> = vec![];

    // BLE
    if let Some(adapter) = bluest::Adapter::default().await {
        match adapter.wait_available().await {
            Ok(()) => {
                let ble_devices: Vec<Device> = stream::iter(adapter.connected_devices().await?)
                    .filter_map(|d| async move {
                        match d.is_paired().await {
                            Ok(false) => {
                                return None;
                            }
                            Err(e) => {
                                tracing::error!(
                                    "can't obtain BLE device pair status for {}: {}",
                                    d.id(),
                                    e
                                );
                                return None;
                            }
                            _ => {}
                        }

                        if !d.is_connected().await {
                            return None;
                        }

                        match d.name() {
                            Ok(name) => Some(Device::Ble {
                                name,
                                address: d.id().to_string(),
                            }),
                            Err(e) => {
                                tracing::error!(
                                    "can't obtain BLE device name for {}: {}",
                                    d.id(),
                                    e
                                );
                                None
                            }
                        }
                    })
                    .collect()
                    .await;

                devices.extend(ble_devices);
            }
            Err(e) => {
                tracing::error!("can't fetch BLE devices: {}", e);
            }
        }
    } else {
        tracing::warn!(
            "can't fetch BLE devices, possible reasons: no bluetooth adapter, bluetooth is turned off, permission denied"
        );
    }

    // Serial
    match meshtastic::utils::stream::available_serial_ports() {
        Ok(ports) => {
            let serial_devices = ports.iter().map(|port| Device::Serial(port.to_owned()));

            devices.extend(serial_devices);
        }
        Err(e) => {
            tracing::error!("can't fetch serial ports: {}", e);
        }
    };

    Ok(devices)
}
