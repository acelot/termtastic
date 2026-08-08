use chrono::Utc;
use meshtastic::{
    Message as _,
    protobufs::{PortNum, Routing, from_radio::PayloadVariant, mesh_packet, routing},
};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::OptionExt;

use crate::types::{Chat, UNKNOWN_NODE};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent, TextMessage},
    state::StateAction,
    types::{AppEvent, Message, Toast},
};
use crate::{state::State, ui::helpers::DateTimeExt};

pub struct ChatService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl ChatService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
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
                event = self.app_event_rx.recv() => self.handle_app_event(event)?,
                event = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        match event {
            Ok(app_event) => match app_event {
                AppEvent::ChatSelected(chat) => {
                    self.state_action_tx.send(StateAction::ActiveChatSet(chat))?;
                }
                AppEvent::ChatPurgeRequested(chat) => {
                    self.state_action_tx.send(StateAction::ChatPurge(chat))?;

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::success("chat is purged")))?;
                }
                AppEvent::ChatSwitchRequested => {
                    self.state_action_tx.send(StateAction::ActiveChatUnset)?;
                }
                AppEvent::ChatMessageSubmitted { text, reply_message_id } => match state.active_chat {
                    Some(Chat::Channel(channel_id)) => {
                        self.meshtastic_command_tx
                            .send(CommandToMeshtastic::SendBroadcastTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                channel_id,
                                reply_message_id,
                                text: TextMessage::Text(text),
                            })?;
                    }
                    Some(Chat::Direct(node_num)) => {
                        self.meshtastic_command_tx
                            .send(CommandToMeshtastic::SendDirectTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                node_num,
                                reply_message_id,
                                text: TextMessage::Text(text),
                            })?;
                    }
                    _ => {}
                },
                AppEvent::ChatReactionSubmitted {
                    emoji,
                    reply_message_id,
                } => match state.active_chat {
                    Some(Chat::Channel(channel_id)) => {
                        self.meshtastic_command_tx
                            .send(CommandToMeshtastic::SendBroadcastTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                channel_id,
                                reply_message_id,
                                text: TextMessage::Emoji(emoji),
                            })?;
                    }
                    Some(Chat::Direct(node_num)) => {
                        self.meshtastic_command_tx
                            .send(CommandToMeshtastic::SendDirectTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                node_num,
                                reply_message_id,
                                text: TextMessage::Emoji(emoji),
                            })?;
                    }
                    _ => {}
                },
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_event(
        &mut self,
        event: Result<MeshtasticEvent, broadcast::error::RecvError>,
    ) -> anyhow::Result<()> {
        match event {
            Ok(meshtastic_event) => match meshtastic_event {
                MeshtasticEvent::IncomingPacket(packet) => self.handle_meshtastic_packet(*packet)?,
                MeshtasticEvent::MessageRejected(e) => {
                    tracing::error!("message rejected: {}", e);

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("message rejected by node")))?;
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_packet(&mut self, payload_variant: PayloadVariant) -> anyhow::Result<()> {
        match payload_variant {
            PayloadVariant::Packet(packet) => match &packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::RoutingApp => match Routing::decode(&*data.payload) {
                        Ok(Routing {
                            variant: Some(routing::Variant::ErrorReason(e)),
                        }) => {
                            let state = &self.state_rx.borrow();

                            if Some(packet.to) == state.my_node_key {
                                self.state_action_tx.send(StateAction::MessageErrorSet {
                                    message_id: data.request_id,
                                    error: Some(routing::Error::try_from(e).expect("invalid routing error")),
                                    datetime: packet.rx_time.timestamp_to_datetime().unwrap_or(Utc::now()),
                                })?;
                            }
                        }
                        Err(e) => {
                            tracing::debug!("can't decode RoutingApp payload: {:?}", e);
                        }
                        _ => {}
                    },
                    PortNum::TextMessageApp | PortNum::ReplyApp => {
                        match Message::try_from((&packet, data)) {
                            Ok(message) => self.state_action_tx.send(StateAction::MessageAdd(message))?,
                            Err(e) => tracing::warn!(
                                packet_id = packet.id,
                                node_from = packet.from,
                                node_to = packet.to,
                                channel = packet.channel,
                                "can't convert packet into message: {}",
                                e
                            ),
                        };
                    }
                    PortNum::RangeTestApp => {
                        let state = &self.state_rx.borrow();
                        let text = String::from_utf8(data.payload.clone()).unwrap_or("can't decode payload".to_owned());
                        let node = state.nodes.get(&packet.from).unwrap_or(&UNKNOWN_NODE);

                        tracing::info!(
                            packet_id = packet.id,
                            node_from = packet.from,
                            node_to = packet.to,
                            channel = packet.channel,
                            "RANGE TEST from [{}] {} ({}), text: \"{}\", hops: {}, snr: {}, rssi: {}",
                            node.short_name(),
                            node.long_name(),
                            node.hw_model(),
                            text,
                            packet.hop_start.saturating_sub(packet.hop_limit),
                            packet.rx_snr,
                            packet.rx_rssi,
                        );
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }
}
