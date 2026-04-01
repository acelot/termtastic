use meshtastic::{
    Message as _,
    protobufs::{PortNum, Routing, from_radio::PayloadVariant, mesh_packet, routing},
};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::{OptionExt, ResultExt};

use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
    types::{AppEvent, Channel, ChannelRole, Message, Node},
};

pub struct ChatService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl ChatService {
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
        loop {
            tokio::select! {
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event)?,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: AppEvent) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        match event {
            AppEvent::ChannelSelected(number) => {
                self.state_action_tx
                    .send(StateAction::ChannelActiveSet(number))?;
            }
            AppEvent::SwitchChannelRequested => {
                self.state_action_tx.send(StateAction::ChannelActiveUnset)?;
            }
            AppEvent::ChatMessageSubmitted(text) => match state.get_active_channel() {
                Some(Channel {
                    key,
                    role: ChannelRole::Primary | ChannelRole::Secondary,
                    ..
                }) => {
                    self.meshtastic_command_tx.send(
                        CommandToMeshtastic::SendBroadcastTextMessage {
                            my_node_id: state.my_node_key.unwrap_or_log(),
                            channel_id: *key,
                            reply_message_id: None,
                            text,
                        },
                    )?;
                }
                Some(Channel {
                    key,
                    role: ChannelRole::Direct,
                    ..
                }) => {
                    self.meshtastic_command_tx.send(
                        CommandToMeshtastic::SendDirectTextMessage {
                            my_node_id: state.my_node_key.unwrap_or_log(),
                            node_id: *key,
                            reply_message_id: None,
                            text,
                        },
                    )?;
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_event(&mut self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            MeshtasticEvent::IncomingPacket(packet) => self.handle_meshtastic_packet(packet)?,
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_packet(&mut self, payload_variant: PayloadVariant) -> anyhow::Result<()> {
        match payload_variant {
            PayloadVariant::Channel(ch) => {
                self.state_action_tx.send(StateAction::ChannelEnsure(
                    ch.index as u32,
                    Channel::from(&ch),
                ))?;
            }
            PayloadVariant::Packet(packet) => match &packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::RoutingApp => match Routing::decode(&*data.payload) {
                        Ok(Routing {
                            variant: Some(routing::Variant::RouteReply(reply)),
                        }) => {}
                        Ok(Routing { variant: _ }) => {}
                        Err(_) => {}
                    },
                    PortNum::TextMessageApp | PortNum::ReplyApp => {
                        let state = &self.state_rx.borrow();

                        let channel_key = match (packet.from, packet.to, state.my_node_key) {
                            (_, 0 | u32::MAX, _) => packet.channel,
                            (from, to, Some(my)) if to == my => {
                                self.state_action_tx.send(StateAction::ChannelEnsure(
                                    from,
                                    Channel::direct(from),
                                ))?;

                                from
                            }
                            (from, to, Some(my)) if from == my => {
                                self.state_action_tx
                                    .send(StateAction::ChannelEnsure(to, Channel::direct(to)))?;

                                to
                            }
                            _ => return Ok(()),
                        };

                        if data.emoji > 0
                            && let Ok(emoji) = String::from_utf8(data.payload.clone())
                            && !emoji.is_empty()
                        {
                            self.state_action_tx.send(StateAction::MessageReactionAdd {
                                channel_key,
                                message_id: data.reply_id,
                                emoji,
                                node_key: packet.from,
                            })?;

                            return Ok(());
                        }

                        match Message::try_from((&packet, data)) {
                            Ok(message) => self
                                .state_action_tx
                                .send(StateAction::MessageAdd(channel_key, message))
                                .unwrap_or_log(),
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
                        let text = String::from_utf8(data.payload.clone())
                            .unwrap_or("can't decoded payload".to_owned());
                        let unknown_node = Node::unknown();
                        let node = state.nodes.get(&packet.from).unwrap_or(&unknown_node);

                        tracing::info!(
                            packet_id = packet.id,
                            node_from = packet.from,
                            node_to = packet.to,
                            channel = packet.channel,
                            "range test packet from [{}] {} ({}), text: \"{}\", hops: {}, snr: {}, rssi: {}",
                            node.short_name,
                            node.long_name,
                            node.hw_model,
                            text,
                            packet.hop_start - packet.hop_limit,
                            packet.rx_snr,
                            packet.rx_rssi,
                        );
                    }
                    portnum => {
                        tracing::info!(
                            packet_id = packet.id,
                            node_from = packet.from,
                            node_to = packet.to,
                            channel = packet.channel,
                            "unhandled portnum: {}",
                            portnum.as_str_name()
                        );
                    }
                },
                Some(mesh_packet::PayloadVariant::Encrypted(_)) => {
                    tracing::info!(
                        packet_id = packet.id,
                        node_from = packet.from,
                        node_to = packet.to,
                        channel = packet.channel,
                        "encrypted packet – ignore"
                    );
                }
                None => {}
            },
            _ => {}
        }

        Ok(())
    }
}
