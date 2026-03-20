use meshtastic::protobufs::FromRadio;
use tokio::sync::{broadcast, mpsc};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::meshtastic::types::{MeshtasticCommand, MeshtasticEvent};

pub struct RadioService {
    command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
    event_tx: broadcast::Sender<MeshtasticEvent>,
}

impl RadioService {
    pub fn new(
        command_tx: mpsc::UnboundedSender<MeshtasticCommand>,
        event_tx: broadcast::Sender<MeshtasticEvent>,
    ) -> Self {
        Self {
            command_tx,
            event_tx,
        }
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

    fn handle_radio_packet(&self, packet: FromRadio) {}
}
