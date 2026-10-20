use meshtastic::protobufs::{FromRadio, from_radio};
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::meshtastic::types::MeshtasticEvent;

pub struct RadioService {
    event_tx: broadcast::Sender<MeshtasticEvent>,
    resume_rx: mpsc::UnboundedReceiver<()>,
    paused: bool,
}

impl RadioService {
    pub fn new(event_tx: broadcast::Sender<MeshtasticEvent>, resume_rx: mpsc::UnboundedReceiver<()>) -> Self {
        Self {
            event_tx,
            resume_rx,
            paused: false,
        }
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
                Some(()) = self.resume_rx.recv() => {
                    tracing::debug!("radio resumed");
                    self.paused = false;
                }
                maybe_packet = radio_rx.recv(), if !self.paused => {
                    if !self.handle_radio_packet(maybe_packet)? {
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
                    if matches!(payload, from_radio::PayloadVariant::MyInfo(_)) {
                        tracing::debug!("MyInfo packet received – radio paused");
                        self.paused = true;
                    }

                    self.event_tx.send(MeshtasticEvent::IncomingPacket(Box::new(payload)))?;
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
