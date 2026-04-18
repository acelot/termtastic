use std::time::Duration;

use meshtastic::{
    api::ConnectedStreamApi,
    packet::{PacketDestination, PacketRouter},
    protobufs::{Config, FromRadio, ModuleConfig, PortNum, from_radio},
    types::{EncodedMeshPacketData, MeshChannel, NodeId},
};
use tokio::{
    sync::{
        broadcast::{self, error::SendError},
        mpsc,
    },
    time::timeout,
};
use tokio_graceful_shutdown::{ErrorAction, NestedSubsystem, SubsystemBuilder, SubsystemHandle};
use tracing_unwrap::OptionExt;

use crate::meshtastic::{
    RadioService, connect_via_ble, connect_via_serial, connect_via_tcp,
    types::{CommandToMeshtastic, MeshtasticEvent, TextMessage},
};

const CONNECTION_TIMEOUT_SECS: u64 = 2;
const SAVE_CONFIG_TIMEOUT_SECS: u64 = 2;

pub struct MeshtasticService {
    command_rx: mpsc::UnboundedReceiver<CommandToMeshtastic>,
    event_tx: broadcast::Sender<MeshtasticEvent>,
    event_rx: broadcast::Receiver<MeshtasticEvent>,
    stream_api: Option<ConnectedStreamApi>,
    radio_subsys: Option<NestedSubsystem>,
}

impl MeshtasticService {
    pub fn new() -> (
        Self,
        mpsc::UnboundedSender<CommandToMeshtastic>,
        broadcast::Receiver<MeshtasticEvent>,
    ) {
        let (command_tx, command_rx) = mpsc::unbounded_channel::<CommandToMeshtastic>();
        let (event_tx, event_rx) = broadcast::channel::<MeshtasticEvent>(100);

        (
            Self {
                command_rx,
                event_tx: event_tx.clone(),
                event_rx: event_rx.resubscribe(),
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
                Ok(event) = self.event_rx.recv() => self.handle_meshtastic_event(event).await?,
                Some(cmd) = self.command_rx.recv() => self.handle_command(cmd, subsys).await?,
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

    async fn handle_meshtastic_event(&mut self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            MeshtasticEvent::RadioStopped => {
                self.disconnect().await?;

                self.event_tx.send(MeshtasticEvent::ConnectionError(
                    "connection channel was closed unexpectedly".to_owned(),
                ))?;
            }
            MeshtasticEvent::IncomingPacket(from_radio::PayloadVariant::Config(Config {
                payload_variant: Some(variant),
            })) => {
                self.event_tx
                    .send(MeshtasticEvent::IncomingConfig(variant))?;
            }
            MeshtasticEvent::IncomingPacket(from_radio::PayloadVariant::ModuleConfig(
                ModuleConfig {
                    payload_variant: Some(variant),
                },
            )) => {
                self.event_tx
                    .send(MeshtasticEvent::IncomingModuleConfig(variant))?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_command(
        &mut self,
        cmd: CommandToMeshtastic,
        subsys: &mut SubsystemHandle,
    ) -> anyhow::Result<()> {
        match cmd {
            CommandToMeshtastic::ConnectViaTcp(hostaddr) => {
                match timeout(
                    Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                    connect_via_tcp(hostaddr),
                )
                .await
                {
                    Ok(Ok((radio_rx, stream_api))) => {
                        self.handle_connection(radio_rx, stream_api, subsys);
                        self.event_tx.send(MeshtasticEvent::Connected)?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("can't connect via TCP: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                    }
                    Err(e) => {
                        tracing::error!("connection timeout: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                    }
                };
            }
            CommandToMeshtastic::ConnectViaBle(address) => {
                match timeout(
                    Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                    connect_via_ble(address),
                )
                .await
                {
                    Ok(Ok((radio_rx, stream_api))) => {
                        self.handle_connection(radio_rx, stream_api, subsys);

                        self.event_tx.send(MeshtasticEvent::Connected)?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("can't connect via BLE: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                    }
                    Err(e) => {
                        tracing::error!("connection timeout: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                    }
                };
            }
            CommandToMeshtastic::ConnectViaSerial(address) => {
                match timeout(
                    Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                    connect_via_serial(address),
                )
                .await
                {
                    Ok(Ok((radio_rx, stream_api))) => {
                        self.handle_connection(radio_rx, stream_api, subsys);
                        self.event_tx.send(MeshtasticEvent::Connected)?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("can't connect via serial: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                    }
                    Err(e) => {
                        tracing::error!("connection timeout: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConnectionError(e.to_string()))?;
                    }
                };
            }
            CommandToMeshtastic::Disconnect => {
                self.disconnect().await?;
                self.event_tx.send(MeshtasticEvent::Disconnected)?;
            }
            CommandToMeshtastic::SendBroadcastTextMessage {
                my_node_id,
                channel_id,
                reply_message_id,
                text,
            } => {
                match self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected")
                    .send_mesh_packet(
                        &mut LocalPacketRouter {
                            my_node_id,
                            event_tx: &self.event_tx,
                        },
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
                    Err(e) => self
                        .event_tx
                        .send(MeshtasticEvent::MessageRejected(e.to_string()))?,
                };
            }
            CommandToMeshtastic::SendDirectTextMessage {
                my_node_id,
                node_id,
                reply_message_id,
                text,
            } => {
                match self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected")
                    .send_mesh_packet(
                        &mut LocalPacketRouter {
                            my_node_id,
                            event_tx: &self.event_tx,
                        },
                        EncodedMeshPacketData::new(match &text {
                            TextMessage::Text(v) => v.clone().into_bytes(),
                            TextMessage::Emoji(e) => e.glyph.as_bytes().to_vec(),
                        }),
                        PortNum::TextMessageApp,
                        PacketDestination::Node(NodeId::from(node_id)),
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
                    Err(e) => self
                        .event_tx
                        .send(MeshtasticEvent::MessageRejected(e.to_string()))?,
                };
            }
            CommandToMeshtastic::SaveConfig { my_node_id, config } => {
                let api = self
                    .stream_api
                    .as_mut()
                    .expect_or_log("should be connected");

                match timeout(Duration::from_secs(SAVE_CONFIG_TIMEOUT_SECS), async {
                    api.start_config_transaction().await?;

                    api.update_config(
                        &mut LocalPacketRouter {
                            my_node_id,
                            event_tx: &self.event_tx,
                        },
                        Config {
                            payload_variant: Some(config),
                        },
                    )
                    .await?;

                    api.commit_config_transaction().await
                })
                .await
                {
                    Ok(Ok(_)) => {
                        self.event_tx.send(MeshtasticEvent::ConfigSaved)?;
                    }
                    Ok(Err(e)) => {
                        tracing::error!("save config error: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConfigSaveError(e.to_string()))?;
                    }
                    Err(e) => {
                        tracing::error!("save config timeout: {:?}", e);

                        self.event_tx
                            .send(MeshtasticEvent::ConfigSaveError(e.to_string()))?;
                    }
                }
            }
        };

        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
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

    fn handle_connection(
        &mut self,
        radio_rx: mpsc::UnboundedReceiver<FromRadio>,
        stream_api: ConnectedStreamApi,
        subsys: &mut SubsystemHandle,
    ) {
        self.stream_api = Some(stream_api);

        let event_tx = self.event_tx.clone();

        let subsys = subsys.start(
            SubsystemBuilder::new(
                "RadioService",
                async |nested_subsys: &mut SubsystemHandle| {
                    RadioService::new(event_tx)
                        .run(radio_rx, nested_subsys)
                        .await
                },
            )
            .on_failure(ErrorAction::CatchAndLocalShutdown),
        );

        self.radio_subsys = Some(subsys);
    }
}

struct LocalPacketRouter<'a> {
    pub my_node_id: u32,
    pub event_tx: &'a broadcast::Sender<MeshtasticEvent>,
}

#[derive(thiserror::Error, Debug)]
enum LocalPacketRouterErr {
    #[error("event send error: {0}")]
    EventSendError(#[from] SendError<MeshtasticEvent>),
}

impl<'a> PacketRouter<(), LocalPacketRouterErr> for LocalPacketRouter<'a> {
    fn handle_packet_from_radio(
        &mut self,
        _packet: meshtastic::protobufs::FromRadio,
    ) -> Result<(), LocalPacketRouterErr> {
        todo!("not implemented")
    }

    fn handle_mesh_packet(
        &mut self,
        packet: meshtastic::protobufs::MeshPacket,
    ) -> Result<(), LocalPacketRouterErr> {
        self.event_tx.send(MeshtasticEvent::IncomingPacket(
            from_radio::PayloadVariant::Packet(packet),
        ))?;

        Ok(())
    }

    fn source_node_id(&self) -> meshtastic::types::NodeId {
        NodeId::new(self.my_node_id)
    }
}
