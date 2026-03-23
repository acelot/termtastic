use meshtastic::protobufs::FromRadio;
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

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
                Some(p) = radio_rx.recv() => self.handle_radio_packet(p),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_radio_packet(&self, packet: FromRadio) {
        if let Some(payload) = packet.payload_variant {
            self.event_tx
                .send(MeshtasticEvent::IncomingPacket(payload))
                .unwrap_or_log();
        }
    }
}
