use std::net::IpAddr;
use std::time::{Duration, Instant};

use hostaddr::HostAddr;
use meshtastic::protobufs::from_radio;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::State;
use crate::types::{AppEvent, Channel, ConnectionState, Device, Toast};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::StateAction,
};

const CONNECTION_CHECK_INTERVAL_MILLIS: u64 = 250;
const RECONNECTION_BACKOFF_BASE_MILLIS: u64 = 1_000;
const RECONNECTION_BACKOFF_MAX_MILLIS: u64 = 30_000;
const BLE_DISCOVERY_TIMEOUT_SECS: u64 = 5;
const MDNS_DISCOVERY_TIMEOUT_SECS: u64 = 5;
const MDNS_MESHTASTIC_DOMAIN: &str = "_meshtastic._tcp.local.";

pub struct ConnectionService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    my_node_key: Option<u32>,
}

impl ConnectionService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            app_event_rx,
            state_rx,
            state_action_tx,
            stored_action_rx,
            meshtastic_command_tx,
            meshtastic_event_rx,
            my_node_key: None,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut connection_check_interval =
            tokio::time::interval(Duration::from_millis(CONNECTION_CHECK_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => self.handle_stored_action(event),
                event = self.app_event_rx.recv() => self.handle_app_event(event).await,
                event = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = connection_check_interval.tick() => self.check_connection(),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_stored_action(&self, event: Result<StateAction, broadcast::error::RecvError>) {
        match event {
            Ok(StateAction::DbLoadSuccess | StateAction::DbLoadFail) => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::ResumeRadio);
                let _ = self.state_action_tx.send(StateAction::ConnectionLoadNodeConfig);

                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::normal("loading node config...")));
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }

    async fn handle_app_event(&mut self, event: Result<AppEvent, broadcast::error::RecvError>) {
        match event {
            Ok(app_event) => match app_event {
                AppEvent::DeviceSelected(hardware) => {
                    let _ = self.state_action_tx.send(StateAction::DeviceActiveSet(hardware));
                }
                AppEvent::DisconnectionRequested => {
                    let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::Disconnect);
                }
                AppEvent::DeviceRediscoverRequested => {
                    if self.state_rx.borrow().device_discovering_state.is_scanning() {
                        return;
                    }

                    let _ = self.state_action_tx.send(StateAction::DeviceDiscoveringStart);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("discovering...")));

                    self.discover_devices();
                }
                AppEvent::DeviceRebootRequested => {
                    let my_node_num = self.state_rx.borrow().my_node_key.expect("should be Some");

                    let _ = self
                        .meshtastic_command_tx
                        .send(CommandToMeshtastic::Reboot { my_node_num, secs: 3 });
                }
                AppEvent::DeviceShutdownRequested => {
                    let my_node_num = self.state_rx.borrow().my_node_key.expect("should be Some");

                    let _ = self
                        .meshtastic_command_tx
                        .send(CommandToMeshtastic::Shutdown { my_node_num, secs: 3 });
                }
                AppEvent::DeviceSubmitted(device) => {
                    let _ = self.state_action_tx.send(StateAction::DevicesAdd(device));
                }
                AppEvent::DeviceRemoveRequested(device) => {
                    let _ = self.state_action_tx.send(StateAction::DevicesRemove(device));
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }

    fn handle_meshtastic_event(&mut self, event: Result<MeshtasticEvent, broadcast::error::RecvError>) {
        match event {
            Ok(meshtastic_event) => match meshtastic_event {
                MeshtasticEvent::ConnectionError(e) => {
                    let _ = self.state_action_tx.send(StateAction::ConnectionFail(e));
                }
                MeshtasticEvent::Disconnected => {
                    tracing::info!("disconnected");

                    let _ = self.state_action_tx.send(StateAction::ConnectionStop);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("disconnected")));
                }
                MeshtasticEvent::IncomingPacket(packet) => {
                    let packet = &*packet;

                    match packet {
                        from_radio::PayloadVariant::MyInfo(my_info) => {
                            self.my_node_key = Some(my_info.my_node_num);

                            let _ = self.state_action_tx.send(StateAction::MyNodeInfoSet {
                                node_key: my_info.my_node_num,
                                nodes_count: my_info.nodedb_count,
                            });
                        }
                        from_radio::PayloadVariant::Metadata(metadata) => {
                            let _ = self
                                .state_action_tx
                                .send(StateAction::DeviceMetadataSet(metadata.clone()));
                        }
                        from_radio::PayloadVariant::Channel(ch) => {
                            let _ = self
                                .state_action_tx
                                .send(StateAction::ChannelSet(ch.index as u32, Channel::from(ch)));
                        }
                        from_radio::PayloadVariant::ConfigCompleteId(_) => {
                            let _ = self.state_action_tx.send(StateAction::ConnectionSuccess);

                            if let Some(my_node_key) = self.my_node_key.take() {
                                let _ = self
                                    .meshtastic_command_tx
                                    .send(CommandToMeshtastic::LoadCannedMessages {
                                        my_node_num: my_node_key,
                                    });
                            }

                            let _ = self
                                .state_action_tx
                                .send(StateAction::Toast(Toast::success("connected")));
                        }
                        from_radio::PayloadVariant::Rebooted(true) => {
                            let _ = self
                                .state_action_tx
                                .send(StateAction::Toast(Toast::success("device has been rebooted")));
                        }
                        from_radio::PayloadVariant::Packet(mesh_packet) => {
                            let state = self.state_rx.borrow();
                            let from = state.nodes.get(&mesh_packet.from).map(|n| n.short_name());
                            let to = state.nodes.get(&mesh_packet.to).map(|n| n.short_name());

                            tracing::debug!("MESH PACKET from=\"{:?}\" to=\"{:?}\": {:?}", from, to, mesh_packet);
                        }
                        _ => {}
                    }

                    let _ = self.state_action_tx.send(StateAction::RxIndicatorTrigger);

                    if !matches!(packet, from_radio::PayloadVariant::Packet(_)) {
                        tracing::debug!("PACKET {:?}", packet);
                    }
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }

    fn check_connection(&self) {
        let state = self.state_rx.borrow();

        match (&state.active_device, &state.connection_state) {
            (Some(device), ConnectionState::ProblemDetected { since, .. }) => {
                let backoff_duration = Duration::from_millis(
                    RECONNECTION_BACKOFF_BASE_MILLIS
                        .saturating_mul(2_u64.saturating_pow(state.connection_attempt))
                        .min(RECONNECTION_BACKOFF_MAX_MILLIS),
                );

                let time_left = backoff_duration.saturating_sub(Instant::now().duration_since(*since));

                let _ = self
                    .state_action_tx
                    .send(StateAction::ReconnectionBackoffSet(time_left));

                if time_left.is_zero() {
                    self.connect(device);
                }
            }
            (Some(device), ConnectionState::NotConnected) => self.connect(device),
            _ => {}
        }
    }

    fn connect(&self, device: &Device) {
        let _ = self.state_action_tx.send(StateAction::ConnectionStart);

        let _ = match device {
            Device::Tcp { address, .. } => self
                .meshtastic_command_tx
                .send(CommandToMeshtastic::ConnectViaTcp(address.clone())),
            Device::Ble { address, id, .. } => self
                .meshtastic_command_tx
                .send(CommandToMeshtastic::ConnectViaBle(*address, id.clone())),
            Device::Serial { address, .. } => self
                .meshtastic_command_tx
                .send(CommandToMeshtastic::ConnectViaSerial(address.to_owned())),
        };
    }

    fn discover_devices(&self) {
        let state_action_tx = self.state_action_tx.clone();

        tokio::spawn(async move {
            let (r1, r2, r3) = tokio::join!(
                discover_serial_devices(state_action_tx.clone()),
                discover_ble_devices(state_action_tx.clone()),
                discover_tcp_devices(state_action_tx.clone()),
            );

            if let Err(e) = r1 {
                tracing::error!("Serial devices discovering failed: {}", e);

                let _ = state_action_tx.send(StateAction::Toast(Toast::error("Serial devices discovering failed")));
            }

            if let Err(e) = r2 {
                tracing::error!("BLE devices discovering failed: {}", e);

                let _ = state_action_tx.send(StateAction::Toast(Toast::error("BLE devices discovering failed")));
            }

            if let Err(e) = r3 {
                tracing::error!("TCP devices discovering failed: {}", e);

                let _ = state_action_tx.send(StateAction::Toast(Toast::error("TCP devices discovering failed")));
            }

            let _ = state_action_tx.send(StateAction::DeviceDiscoveringDone);
        });
    }
}

async fn discover_serial_devices(state_action_tx: mpsc::UnboundedSender<StateAction>) -> anyhow::Result<()> {
    tokio::spawn(async move {
        let ports = meshtastic::utils::stream::available_serial_ports()?;
        let serial_devices = ports.iter().map(|port| Device::Serial {
            address: port.to_owned(),
            name: None,
        });

        for device in serial_devices.into_iter() {
            let _ = state_action_tx.send(StateAction::DevicesDiscoveredAdd(device));
        }

        Ok(())
    })
    .await?
}

async fn discover_ble_devices(state_action_tx: mpsc::UnboundedSender<StateAction>) -> anyhow::Result<()> {
    let devices =
        meshtastic::utils::stream::available_ble_devices(Duration::from_secs(BLE_DISCOVERY_TIMEOUT_SECS)).await?;

    let ble_devices = devices.iter().map(|device| Device::Ble {
        address: device.mac_address,
        id: device.name.clone(),
        name: None,
    });

    for device in ble_devices.into_iter() {
        let _ = state_action_tx.send(StateAction::DevicesDiscoveredAdd(device));
    }

    Ok(())
}

async fn discover_tcp_devices(state_action_tx: mpsc::UnboundedSender<StateAction>) -> anyhow::Result<()> {
    let mdns_daemon = mdns_sd::ServiceDaemon::new()?;
    let receiver = mdns_daemon.browse(MDNS_MESHTASTIC_DOMAIN)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(MDNS_DISCOVERY_TIMEOUT_SECS);

    let result: anyhow::Result<()> = 'browse: loop {
        let event = match tokio::time::timeout_at(deadline, receiver.recv_async()).await {
            Ok(Ok(event)) => event,
            Ok(Err(_)) => break Ok(()),
            Err(_) => {
                tracing::debug!("mDNS timeout, stopping discovery");
                break Ok(());
            }
        };

        let mdns_sd::ServiceEvent::ServiceResolved(info) = event else {
            continue;
        };

        let short_name = info.txt_properties.get("shortname").map(|txt| txt.val_str().to_owned());

        for addr in info.addresses.iter() {
            let ip = match addr {
                mdns_sd::ScopedIp::V4(ipv4) => IpAddr::from(*ipv4.addr()),
                mdns_sd::ScopedIp::V6(ipv6) => IpAddr::from(*ipv6.addr()),
                _ => continue,
            };

            if let Err(e) = state_action_tx.send(StateAction::DevicesDiscoveredAdd(Device::Tcp {
                address: HostAddr::from_ip_addr(ip).with_port(info.port),
                name: short_name.clone(),
            })) {
                break 'browse Err(e.into());
            }
        }
    };

    if result.is_err() {
        let _ = mdns_daemon.shutdown();
        return result;
    }

    mdns_daemon.shutdown()?;

    Ok(())
}
