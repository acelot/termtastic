use meshtastic::{
    Message as _,
    protobufs::{PortNum, Routing, from_radio::PayloadVariant, mesh_packet, routing},
};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
    types::{AppEvent, Channel, Message},
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
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event),
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_app_event(&self, event: AppEvent) {
        match event {
            AppEvent::ChannelSelected(number) => {
                self.state_action_tx
                    .send(StateAction::ChannelActiveSet(number))
                    .unwrap_or_log();
            }
            AppEvent::SwitchChannelRequested => {
                self.state_action_tx
                    .send(StateAction::ChannelActiveUnset)
                    .unwrap_or_log();
            }
            _ => {}
        }
    }

    fn handle_meshtastic_event(&mut self, event: MeshtasticEvent) {
        match event {
            MeshtasticEvent::IncomingPacket(packet) => self.handle_meshtastic_packet(packet),
            _ => {}
        }
    }

    fn handle_meshtastic_packet(&mut self, payload_variant: PayloadVariant) {
        match payload_variant {
            PayloadVariant::Channel(ch) => {
                self.state_action_tx
                    .send(StateAction::ChannelEnsure(
                        ch.index as u32,
                        Channel::from(&ch),
                    ))
                    .unwrap_or_log();
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

                        let channel_key = match (packet.to, state.my_node_number) {
                            (0 | u32::MAX, _) => packet.channel,
                            (to, Some(my)) if to == my && to != packet.from => {
                                self.state_action_tx
                                    .send(StateAction::ChannelEnsure(
                                        packet.from,
                                        Channel::direct(packet.from),
                                    ))
                                    .unwrap_or_log();

                                packet.from
                            }
                            _ => return,
                        };

                        if data.emoji != 0
                            && let Some(emoji) = char::from_u32(data.emoji)
                        {
                            self.state_action_tx
                                .send(StateAction::MessageReactionAdd {
                                    channel_key,
                                    message_id: packet.id,
                                    emoji,
                                    node_key: packet.from,
                                })
                                .unwrap_or_log();

                            return;
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
    }
}
