use meshtastic::protobufs::FromRadio;
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::meshtastic::types::MeshtasticEvent;

pub struct RadioService {
    event_tx: broadcast::Sender<MeshtasticEvent>,
}

impl RadioService {
    pub fn new(event_tx: broadcast::Sender<MeshtasticEvent>) -> Self {
        Self { event_tx }
    }

    pub async fn run(
        &mut self,
        mut radio_rx: mpsc::UnboundedReceiver<FromRadio>,
        subsys: &mut SubsystemHandle,
    ) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                _ = subsys.on_shutdown_requested() => {
                    return Ok(());
                }
                maybe_packet = radio_rx.recv() => {
                    if self.handle_radio_packet(maybe_packet)? == false {
                        tracing::warn!("radio channel was closed unexpectedly");
                        return Ok(());
                    }
                }
            }
        }
    }

    fn handle_radio_packet(&mut self, maybe_packet: Option<FromRadio>) -> anyhow::Result<bool> {
        match maybe_packet {
            Some(packet) => {
                if let Some(payload) = packet.payload_variant {
                    self.event_tx
                        .send(MeshtasticEvent::IncomingPacket(payload))?;
                }

                Ok(true)
            }
            None => {
                self.event_tx.send(MeshtasticEvent::RadioStopped)?;

                Ok(false)
            }
        }
    }
}
