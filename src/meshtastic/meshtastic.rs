use meshtastic::{
    Message,
    api::ConnectedStreamApi,
    packet::{PacketDestination, PacketRouter},
    protobufs::{Config, FromRadio, MeshPacket, ModuleConfig, PortNum, RouteDiscovery, admin_message, from_radio},
    types::{EncodedMeshPacketData, MeshChannel, NodeId},
};
use std::convert::Infallible;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio::{
    sync::{broadcast, mpsc},
    time::timeout,
};
use tokio_graceful_shutdown::{ErrorAction, NestedSubsystem, SubsystemBuilder, SubsystemHandle};
use tracing_unwrap::OptionExt;

use crate::meshtastic::{
    RadioService, connect_via_ble, connect_via_serial, connect_via_tcp,
    types::{CommandToMeshtastic, MeshtasticEvent, TextMessage},
};

const BLE_CONNECTION_TIMEOUT_SECS: u64 = 60;
const TCP_CONNECTION_TIMEOUT_SECS: u64 = 5;
const SERIAL_CONNECTION_TIMEOUT_SECS: u64 = 5;
const SAVE_CONFIG_TIMEOUT_SECS: u64 = 5;
const SAVE_CONFIG_DELAY_MILLIS: u64 = 100;

pub struct MeshtasticService {
    command_rx: mpsc::UnboundedReceiver<CommandToMeshtastic>,
    event_tx: broadcast::Sender<MeshtasticEvent>,
    event_rx: broadcast::Receiver<MeshtasticEvent>,
    stream_api: Option<ConnectedStreamApi>,
    radio_subsys: Option<NestedSubsystem>,
    connection_join_handle: Option<JoinHandle<anyhow::Result<()>>>,
    connection_tx: mpsc::Sender<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)>,
    connection_rx: mpsc::Receiver<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)>,
}

impl MeshtasticService {
    pub fn new() -> (
        Self,
        mpsc::UnboundedSender<CommandToMeshtastic>,
        broadcast::Receiver<MeshtasticEvent>,
    ) {
        let (command_tx, command_rx) = mpsc::unbounded_channel::<CommandToMeshtastic>();
        let (event_tx, event_rx) = broadcast::channel::<MeshtasticEvent>(1024);
        let (connection_tx, connection_rx) = mpsc::channel(1);

        (
            Self {
                command_rx,
                event_tx: event_tx.clone(),
                event_rx: event_rx.resubscribe(),
                stream_api: None,
                radio_subsys: None,
                connection_join_handle: None,
                connection_tx,
                connection_rx,
            },
            command_tx.clone(),
            event_rx,
        )
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.event_rx.recv() => self.handle_meshtastic_event(event).await?,
                Some((radio_rx, stream_api)) = self.connection_rx.recv() => {
                    self.handle_connection_event(radio_rx, stream_api, subsys)?;
                },
                Some(cmd) = self.command_rx.recv() => self.handle_command(cmd).await?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    self.disconnect().await?;
                    self.event_tx.send(MeshtasticEvent::Disconnected)?;

                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_meshtastic_event(
        &mut self,
        event: Result<MeshtasticEvent, broadcast::error::RecvError>,
    ) -> anyhow::Result<()> {
        match event {
            Ok(MeshtasticEvent::RadioStopped) => {
                self.disconnect().await?;

                self.event_tx.send(MeshtasticEvent::ConnectionError(
                    "connection channel was closed unexpectedly".to_owned(),
                ))?;
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: CommandToMeshtastic) -> anyhow::Result<()> {
        let connection_tx_clone = self.connection_tx.clone();
        let event_tx_clone = self.event_tx.clone();

        match cmd {
            CommandToMeshtastic::ConnectViaTcp(address) => {
                self.connection_join_handle = Some(tokio::spawn(async move {
                    match timeout(
                        Duration::from_secs(TCP_CONNECTION_TIMEOUT_SECS),
                        connect_via_tcp(address),
                    )
                    .await
                    {
                        Ok(Ok(conn)) => {
                            connection_tx_clone.send(conn).await?;
                        }
                        Ok(Err(e)) => {
                            event_tx_clone.send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                        }
                        Err(e) => {
                            event_tx_clone.send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                        }
                    }

                    Ok(())
                }));
            }
            CommandToMeshtastic::ConnectViaBle(address, name) => {
                self.connection_join_handle = Some(tokio::spawn(async move {
                    match timeout(
                        Duration::from_secs(BLE_CONNECTION_TIMEOUT_SECS),
                        connect_via_ble(address, name),
                    )
                    .await
                    {
                        Ok(Ok(conn)) => {
                            connection_tx_clone.send(conn).await?;
                        }
                        Ok(Err(e)) => {
                            event_tx_clone.send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                        }
                        Err(e) => {
                            event_tx_clone.send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                        }
                    }

                    Ok(())
                }));
            }
            CommandToMeshtastic::ConnectViaSerial(address) => {
                self.connection_join_handle = Some(tokio::spawn(async move {
                    match timeout(
                        Duration::from_secs(SERIAL_CONNECTION_TIMEOUT_SECS),
                        connect_via_serial(address),
                    )
                    .await
                    {
                        Ok(Ok(conn)) => {
                            connection_tx_clone.send(conn).await?;
                        }
                        Ok(Err(e)) => {
                            event_tx_clone.send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                        }
                        Err(e) => {
                            event_tx_clone.send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                        }
                    }

                    Ok(())
                }));
            }
            CommandToMeshtastic::Disconnect => {
                self.disconnect().await?;
                self.event_tx.send(MeshtasticEvent::Disconnected)?;
            }
            CommandToMeshtastic::Reboot { secs, my_node_num } => {
                self.send_admin_message(my_node_num, admin_message::PayloadVariant::RebootSeconds(secs))
                    .await?;
            }
            CommandToMeshtastic::Shutdown { secs, my_node_num } => {
                self.send_admin_message(my_node_num, admin_message::PayloadVariant::ShutdownSeconds(secs))
                    .await?;
            }
            CommandToMeshtastic::LoadCannedMessages { my_node_num } => {
                self.send_admin_message(
                    my_node_num,
                    admin_message::PayloadVariant::GetCannedMessageModuleMessagesRequest(true),
                )
                .await?;
            }
            CommandToMeshtastic::SaveCannedMessages { messages, my_node_num } => {
                self.send_admin_message(
                    my_node_num,
                    admin_message::PayloadVariant::SetCannedMessageModuleMessages(messages),
                )
                .await?;
            }
            CommandToMeshtastic::SendBroadcastTextMessage {
                channel_id,
                reply_message_id,
                text,
                my_node_num,
            } => {
                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected")
                    .send_mesh_packet(
                        &mut packet_router,
                        EncodedMeshPacketData::new(match &text {
                            TextMessage::Text(v) => v.clone().into_bytes(),
                            TextMessage::Emoji(e) => e.glyph.as_bytes().to_vec(),
                        }),
                        PortNum::TextMessageApp,
                        PacketDestination::Broadcast,
                        MeshChannel::from(channel_id),
                        true,                                               // want_ack
                        false,                                              // want_response
                        true,                                               // echo_response
                        reply_message_id,                                   // reply_id
                        matches!(text, TextMessage::Emoji(_)).then_some(1), // emoji
                    )
                    .await
                {
                    Ok(()) => self.event_tx.send(MeshtasticEvent::MessageAccepted)?,
                    Err(e) => self.event_tx.send(MeshtasticEvent::MessageRejected(e.to_string()))?,
                };
            }
            CommandToMeshtastic::SendDirectTextMessage {
                node_num,
                reply_message_id,
                text,
                my_node_num,
            } => {
                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected")
                    .send_mesh_packet(
                        &mut packet_router,
                        EncodedMeshPacketData::new(match &text {
                            TextMessage::Text(v) => v.clone().into_bytes(),
                            TextMessage::Emoji(e) => e.glyph.as_bytes().to_vec(),
                        }),
                        PortNum::TextMessageApp,
                        PacketDestination::Node(NodeId::from(node_num)),
                        MeshChannel::from(0),
                        true,                                               // want_ack
                        false,                                              // want_response
                        true,                                               // echo_response
                        reply_message_id,                                   // reply_id
                        matches!(text, TextMessage::Emoji(_)).then_some(1), // emoji
                    )
                    .await
                {
                    Ok(()) => self.event_tx.send(MeshtasticEvent::MessageAccepted)?,
                    Err(e) => self.event_tx.send(MeshtasticEvent::MessageRejected(e.to_string()))?,
                };
            }
            CommandToMeshtastic::SaveConfig {
                form_id,
                config,
                my_node_num,
            } => {
                let api = self.stream_api.as_mut().expect_or_log("should be connected");

                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match async {
                    api.start_config_transaction().await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    api.update_config(
                        &mut packet_router,
                        Config {
                            payload_variant: Some(config),
                        },
                    )
                    .await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    api.commit_config_transaction().await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    Ok::<(), anyhow::Error>(())
                }
                .await
                {
                    Ok(_) => {
                        self.event_tx.send(MeshtasticEvent::ConfigSaved(form_id))?;
                    }
                    Err(e) => {
                        tracing::error!("save config error: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::ConfigSaveFailed(form_id))?;
                    }
                }
            }
            CommandToMeshtastic::SaveModuleConfig {
                form_id,
                config,
                my_node_num,
            } => {
                let api = self.stream_api.as_mut().expect_or_log("should be connected");

                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match timeout(Duration::from_secs(SAVE_CONFIG_TIMEOUT_SECS), async {
                    api.start_config_transaction().await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    api.update_module_config(
                        &mut packet_router,
                        ModuleConfig {
                            payload_variant: Some(config),
                        },
                    )
                    .await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    api.commit_config_transaction().await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    Ok::<(), anyhow::Error>(())
                })
                .await
                {
                    Ok(Ok(_)) => {
                        self.event_tx.send(MeshtasticEvent::ConfigSaved(form_id))?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("save config error: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::ConfigSaveFailed(form_id))?;
                    }
                    Err(e) => {
                        tracing::error!("save config timeout: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::ConfigSaveFailed(form_id))?;
                    }
                }
            }
            CommandToMeshtastic::SaveUser {
                form_id,
                user,
                my_node_num,
            } => {
                let api = self.stream_api.as_mut().expect_or_log("should be connected");

                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match timeout(Duration::from_secs(SAVE_CONFIG_TIMEOUT_SECS), async {
                    api.update_user(&mut packet_router, user).await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    Ok::<(), anyhow::Error>(())
                })
                .await
                {
                    Ok(Ok(_)) => {
                        self.event_tx.send(MeshtasticEvent::UserSaved(form_id))?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("save user error: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::UserSaveFailed(form_id))?;
                    }
                    Err(e) => {
                        tracing::error!("save user timeout: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::UserSaveFailed(form_id))?;
                    }
                }
            }
            CommandToMeshtastic::BroadcastNodeInfo { channel_id, user } => {
                match self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected")
                    .send_mesh_packet(
                        &mut NullPacketRouter {},
                        EncodedMeshPacketData::new(user.encode_to_vec().into()),
                        PortNum::NodeinfoApp,
                        PacketDestination::Broadcast,
                        MeshChannel::from(channel_id),
                        false, // want_ack
                        false, // want_response
                        false, // echo_response
                        None,  // reply_id
                        None,  // emoji
                    )
                    .await
                {
                    Ok(()) => self.event_tx.send(MeshtasticEvent::NodeInfoBroadcastSent)?,
                    Err(e) => self
                        .event_tx
                        .send(MeshtasticEvent::NodeInfoBroadcastFailed(e.to_string()))?,
                };
            }
            CommandToMeshtastic::SaveChannelsConfig {
                form_id,
                channels,
                my_node_num,
            } => {
                let api = self.stream_api.as_mut().expect_or_log("should be connected");

                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match timeout(Duration::from_secs(SAVE_CONFIG_TIMEOUT_SECS), async {
                    api.start_config_transaction().await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    for channel in channels {
                        api.update_channel_config(&mut packet_router, channel).await?;
                        sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;
                    }

                    api.commit_config_transaction().await?;

                    sleep(Duration::from_millis(SAVE_CONFIG_DELAY_MILLIS)).await;

                    Ok::<(), anyhow::Error>(())
                })
                .await
                {
                    Ok(Ok(_)) => {
                        self.event_tx.send(MeshtasticEvent::ChannelsSaved(form_id))?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("save channels error: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::ChannelsSaveFailed(form_id))?;
                    }
                    Err(e) => {
                        tracing::error!("save channels timeout: {:?}", e);

                        self.event_tx.send(MeshtasticEvent::ChannelsSaveFailed(form_id))?;
                    }
                }
            }
            CommandToMeshtastic::DeleteNode { node_num, my_node_num } => {
                match self
                    .send_admin_message(my_node_num, admin_message::PayloadVariant::RemoveByNodenum(node_num))
                    .await
                {
                    Ok(()) => self.event_tx.send(MeshtasticEvent::NodeRemoveAccepted)?,
                    Err(e) => self.event_tx.send(MeshtasticEvent::NodeRemoveFailed(e.to_string()))?,
                };
            }
            CommandToMeshtastic::RunTraceroute { node_num, my_node_num } => {
                let mut packet_router = RetransmitPacketRouter {
                    my_node_num,
                    event_tx: &self.event_tx,
                };

                match self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected")
                    .send_mesh_packet(
                        &mut packet_router,
                        EncodedMeshPacketData::new(RouteDiscovery::default().encode_to_vec()),
                        PortNum::TracerouteApp,
                        PacketDestination::Node(NodeId::new(node_num)),
                        MeshChannel::new(0)?,
                        false, // want_ack
                        true,  // want_response
                        true,  // echo_response
                        None,  // reply_id
                        None,  // emoji
                    )
                    .await
                {
                    Ok(()) => self.event_tx.send(MeshtasticEvent::TracerouteStarted)?,
                    Err(e) => self.event_tx.send(MeshtasticEvent::TracerouteFailed(e.to_string()))?,
                };
            }
        };

        Ok(())
    }

    fn handle_connection_event(
        &mut self,
        radio_rx: mpsc::UnboundedReceiver<FromRadio>,
        stream_api: ConnectedStreamApi,
        subsys: &mut SubsystemHandle,
    ) -> anyhow::Result<()> {
        self.stream_api = Some(stream_api);

        let event_tx_clone = self.event_tx.clone();

        self.radio_subsys = Some(
            subsys.start(
                SubsystemBuilder::new("RadioService", async |nested_subsys: &mut SubsystemHandle| {
                    RadioService::new(event_tx_clone).run(radio_rx, nested_subsys).await
                })
                .on_failure(ErrorAction::CatchAndLocalShutdown),
            ),
        );

        self.event_tx.send(MeshtasticEvent::Connected)?;

        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(handle) = self.connection_join_handle.take() {
            handle.abort();
        }

        if let Some(subsys) = self.radio_subsys.take() {
            if !subsys.is_finished() {
                subsys.initiate_shutdown();
                subsys.join().await?;
            }
        }

        if let Some(stream_api) = self.stream_api.take() {
            let _ = stream_api
                .disconnect()
                .await
                .inspect_err(|e| tracing::error!("stream api disconnect error: {}", e));
        }

        Ok(())
    }

    async fn send_admin_message(
        &mut self,
        my_node_num: u32,
        payload: admin_message::PayloadVariant,
    ) -> anyhow::Result<()> {
        let mut packet_router = RetransmitPacketRouter {
            my_node_num,
            event_tx: &self.event_tx,
        };

        let packet = meshtastic::protobufs::AdminMessage {
            payload_variant: Some(payload),
            session_passkey: Vec::new(),
        };

        self.stream_api
            .as_mut()
            .expect_or_log("should be connected")
            .send_mesh_packet(
                &mut packet_router,
                EncodedMeshPacketData::new(packet.encode_to_vec()),
                PortNum::AdminApp,
                PacketDestination::Local,
                MeshChannel::new(0)?,
                false, // want_ack
                true,  // want_response
                true,  // echo_response
                None,  // reply_id
                None,  // emoji
            )
            .await?;

        Ok(())
    }
}

struct NullPacketRouter {}

impl PacketRouter<(), Infallible> for NullPacketRouter {
    fn handle_packet_from_radio(&mut self, _packet: FromRadio) -> Result<(), Infallible> {
        Ok(())
    }

    fn handle_mesh_packet(&mut self, _packet: MeshPacket) -> Result<(), Infallible> {
        Ok(())
    }

    fn source_node_id(&self) -> NodeId {
        NodeId::default()
    }
}

struct RetransmitPacketRouter<'a> {
    pub my_node_num: u32,
    pub event_tx: &'a broadcast::Sender<MeshtasticEvent>,
}

#[derive(thiserror::Error, Debug)]
enum RetransmitPacketRouterErr {
    #[error("event send error: {0}")]
    EventSendError(#[from] broadcast::error::SendError<MeshtasticEvent>),
}

impl<'a> PacketRouter<(), RetransmitPacketRouterErr> for RetransmitPacketRouter<'a> {
    fn handle_packet_from_radio(&mut self, packet: FromRadio) -> Result<(), RetransmitPacketRouterErr> {
        if let Some(payload) = packet.payload_variant {
            self.event_tx.send(MeshtasticEvent::IncomingPacket(payload))?;
        }

        Ok(())
    }

    fn handle_mesh_packet(&mut self, packet: MeshPacket) -> Result<(), RetransmitPacketRouterErr> {
        self.event_tx
            .send(MeshtasticEvent::IncomingPacket(from_radio::PayloadVariant::Packet(
                packet,
            )))?;

        Ok(())
    }

    fn source_node_id(&self) -> NodeId {
        NodeId::new(self.my_node_num)
    }
}
