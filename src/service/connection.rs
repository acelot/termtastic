use std::time::Instant;

use futures::stream::{self, StreamExt};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::types::{AppEvent, ConnectionState, Device, DevicesDiscoveringState};
use crate::{
    meshtastic::types::{MeshtasticCommand, MeshtasticEvent},
    state::{State, StateAction},
};

pub struct ConnectionService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl ConnectionService {
    pub fn new(
        app_event_tx: broadcast::Sender<AppEvent>,
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
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
        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event).await,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = self.state_rx.changed() => self.handle_state_change(),

                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_app_event(&self, event: AppEvent) {
        match event {
            AppEvent::InitializationRequested => {
                self.app_event_tx
                    .send(AppEvent::DeviceRediscoverRequested)
                    .unwrap_or_log();
            }
            AppEvent::DeviceSelected(hardware) => {
                self.state_action_tx
                    .send(StateAction::SetSelectedDevice(hardware))
                    .unwrap_or_log();
            }
            AppEvent::DisconnectionRequested => {
                self.meshtastic_command_tx
                    .send(MeshtasticCommand::Disconnect)
                    .unwrap_or_log();
            }
            AppEvent::DeviceRediscoverRequested => {
                self.state_action_tx
                    .send(StateAction::SetDevicesDiscoveringState(
                        DevicesDiscoveringState::InProgress,
                    ))
                    .unwrap_or_log();

                match discover_devices().await {
                    Ok(devices) => {
                        self.state_action_tx
                            .send(StateAction::SetDiscoveredDevices(devices))
                            .unwrap_or_log();

                        self.state_action_tx
                            .send(StateAction::SetDevicesDiscoveringState(
                                DevicesDiscoveringState::Finished,
                            ))
                            .unwrap_or_log();
                    }
                    Err(e) => self
                        .state_action_tx
                        .send(StateAction::SetDevicesDiscoveringState(
                            DevicesDiscoveringState::Error(e.to_string()),
                        ))
                        .unwrap_or_log(),
                };
            }
            AppEvent::TcpDeviceSubmitted(mut hostaddr) => {
                if !hostaddr.has_port() {
                    hostaddr = hostaddr.with_port(4403);
                }

                self.state_action_tx
                    .send(StateAction::AddTcpDevice(hostaddr))
                    .unwrap_or_log();
            }
            AppEvent::TcpDeviceRemoved(hostaddr) => {
                self.state_action_tx
                    .send(StateAction::RemoveTcpDevice(hostaddr))
                    .unwrap_or_log();
            }
            _ => {}
        }
    }

    fn handle_meshtastic_event(&self, event: MeshtasticEvent) {
        match event {
            MeshtasticEvent::Connected => {
                self.state_action_tx
                    .send(StateAction::SetConnectionState(ConnectionState::Connected))
                    .unwrap_or_log();
            }
            MeshtasticEvent::ConnectionError(e) => {
                self.state_action_tx
                    .send(StateAction::SetConnectionState(
                        ConnectionState::ProblemDetected {
                            since: Instant::now(),
                            error: e,
                        },
                    ))
                    .unwrap_or_log();
            }
            MeshtasticEvent::Disconnected => {
                self.state_action_tx
                    .send(StateAction::SetConnectionState(
                        ConnectionState::NotConnected,
                    ))
                    .unwrap_or_log();

                self.state_action_tx
                    .send(StateAction::UnsetConnection)
                    .unwrap_or_log();
            }
            MeshtasticEvent::IncomingPacket(_) => {
                self.state_action_tx
                    .send(StateAction::TriggerRx)
                    .unwrap_or_log();
            }
            _ => {}
        }
    }

    fn handle_state_change(&self) {
        let state = &self.state_rx.borrow();

        if let Some(device) = &state.app_config.selected_device
            && state.connection_state == ConnectionState::NotConnected
        {
            self.state_action_tx
                .send(StateAction::SetConnectionState(ConnectionState::Connecting))
                .unwrap_or_log();

            match device {
                Device::Tcp(hostaddr) => self
                    .meshtastic_command_tx
                    .send(MeshtasticCommand::ConnectViaTcp(hostaddr.clone()))
                    .unwrap_or_log(),
                Device::Ble { address, .. } => self
                    .meshtastic_command_tx
                    .send(MeshtasticCommand::ConnectViaBle(address.to_owned()))
                    .unwrap_or_log(),
                Device::Serial(address) => self
                    .meshtastic_command_tx
                    .send(MeshtasticCommand::ConnectViaSerial(address.to_owned()))
                    .unwrap_or_log(),
            };
        }
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
            "can't fetch BLE devices, possible reasons:
- no bluetooth adapter
- bluetooth is turned off
- permission denied"
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
