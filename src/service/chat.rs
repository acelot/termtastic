use std::time::Instant;

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
    types::{Message, Toast, UiEvent},
};
use crate::{state::State, ui::helpers::DateTimeExt};

const CHAT_TOP_DEBOUNCE_DELAY_MILLIS: u128 = 1000;

pub struct ChatService {
    ui_event_rx: broadcast::Receiver<UiEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    last_chat_top_t: Instant,
}

impl ChatService {
    pub fn new(
        ui_event_rx: broadcast::Receiver<UiEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            ui_event_rx,
            state_rx,
            state_action_tx,
            meshtastic_command_tx,
            meshtastic_event_rx,
            last_chat_top_t: Instant::now(),
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.ui_event_rx.recv() => self.handle_ui_event(event),
                event = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_ui_event(&mut self, event: Result<UiEvent, broadcast::error::RecvError>) {
        let state = self.state_rx.borrow();

        match event {
            Ok(app_event) => match app_event {
                UiEvent::ChatSelected(chat) => {
                    let _ = self.state_action_tx.send(StateAction::ActiveChatSet(chat));
                }
                UiEvent::ChatPurgeRequested(chat) => {
                    let _ = self.state_action_tx.send(StateAction::ChatPurge(chat));

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::success("chat is purged")));
                }
                UiEvent::ChatSwitchRequested => {
                    let _ = self.state_action_tx.send(StateAction::ActiveChatUnset);
                }
                UiEvent::ChatTopReached => {
                    if self.last_chat_top_t.elapsed().as_millis() > CHAT_TOP_DEBOUNCE_DELAY_MILLIS {
                        let _ = self.state_action_tx.send(StateAction::Toast(Toast::normal(
                            "press <Shift+Up> to load previous messages",
                        )));
                    }

                    self.last_chat_top_t = Instant::now();
                }
                UiEvent::ChatPreviousMessagesLoadRequested => {
                    if let Some(active_chat) = state.active_chat.as_ref().cloned() {
                        let _ = self
                            .state_action_tx
                            .send(StateAction::PreviousMessagesLoadPending(active_chat));
                    }
                }
                UiEvent::ChatMessageSubmitted { text, reply_id } => match state.active_chat {
                    Some(Chat::Channel(channel_id)) => {
                        let _ = self
                            .meshtastic_command_tx
                            .send(CommandToMeshtastic::SendBroadcastTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                channel_id,
                                reply_packet_id: reply_id
                                    .and_then(|id| state.messages.get(&id))
                                    .and_then(|msg| Some(msg.packet_id)),
                                text: TextMessage::Text(text),
                            });
                    }
                    Some(Chat::Direct(node_num)) => {
                        let _ = self
                            .meshtastic_command_tx
                            .send(CommandToMeshtastic::SendDirectTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                node_num,
                                reply_packet_id: reply_id
                                    .and_then(|id| state.messages.get(&id))
                                    .and_then(|msg| Some(msg.packet_id)),
                                text: TextMessage::Text(text),
                            });
                    }
                    _ => {}
                },
                UiEvent::ChatReactionSubmitted { emoji, reply_id } => match state.active_chat {
                    Some(Chat::Channel(channel_id)) => {
                        let _ = self
                            .meshtastic_command_tx
                            .send(CommandToMeshtastic::SendBroadcastTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                channel_id,
                                reply_packet_id: reply_id
                                    .and_then(|id| state.messages.get(&id))
                                    .and_then(|msg| Some(msg.packet_id)),
                                text: TextMessage::Emoji(emoji),
                            });
                    }
                    Some(Chat::Direct(node_num)) => {
                        let _ = self
                            .meshtastic_command_tx
                            .send(CommandToMeshtastic::SendDirectTextMessage {
                                my_node_num: state.my_node_key.expect_or_log("my node key should exists"),
                                node_num,
                                reply_packet_id: reply_id
                                    .and_then(|id| state.messages.get(&id))
                                    .and_then(|msg| Some(msg.packet_id)),
                                text: TextMessage::Emoji(emoji),
                            });
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
    }

    fn handle_meshtastic_event(&mut self, event: Result<MeshtasticEvent, broadcast::error::RecvError>) {
        match event {
            Ok(meshtastic_event) => match meshtastic_event {
                MeshtasticEvent::IncomingPacket(packet) => {
                    self.handle_meshtastic_packet(*packet);
                }
                MeshtasticEvent::MessageRejected(e) => {
                    tracing::error!("message rejected: {}", e);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("message rejected by node")));
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }

    fn handle_meshtastic_packet(&mut self, payload_variant: PayloadVariant) {
        match payload_variant {
            PayloadVariant::Packet(packet) => match &packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::RoutingApp => match Routing::decode(&*data.payload) {
                        Ok(Routing {
                            variant: Some(routing::Variant::ErrorReason(e)),
                        }) => {
                            let my_node_key = self.state_rx.borrow().my_node_key;

                            if Some(packet.to) == my_node_key {
                                match routing::Error::try_from(e) {
                                    Ok(error) => {
                                        let _ = self.state_action_tx.send(StateAction::MessageErrorSet {
                                            packet_id: data.request_id,
                                            error: Some(error),
                                            datetime: packet.rx_time.timestamp_to_datetime().unwrap_or(Utc::now()),
                                        });
                                    }
                                    Err(_) => {
                                        tracing::warn!(
                                            message_id = data.request_id,
                                            "unknown routing error code: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::debug!("can't decode RoutingApp payload: {:?}", e);
                        }
                        _ => {}
                    },
                    PortNum::TextMessageApp | PortNum::ReplyApp => {
                        match Message::try_from((&packet, data)) {
                            Ok(message) => {
                                let _ = self.state_action_tx.send(StateAction::MessageAdd(message));
                            }
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
                        let state = self.state_rx.borrow();
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
    }
}
