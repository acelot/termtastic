use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    watch,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::state::{State, StateAction};

pub struct Store {
    state: State,
    action_rx: UnboundedReceiver<StateAction>,
    state_tx: watch::Sender<State>,
}

impl Store {
    pub fn new(
        initial_state: State,
    ) -> (Self, UnboundedSender<StateAction>, watch::Receiver<State>) {
        let (action_tx, action_rx) = unbounded_channel::<StateAction>();
        let (state_tx, state_rx) = watch::channel(initial_state.clone());

        (
            Self {
                state: initial_state.clone(),
                action_rx,
                state_tx,
            },
            action_tx,
            state_rx,
        )
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                Some(action) = self.action_rx.recv() => self.handle_action(action),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: StateAction) {
        let prev_state = self.state.clone();

        match action {
            StateAction::SetAppConfig(cfg) => {
                self.state.app_config = cfg;
            }
            StateAction::SetAppConfigDevices(cfg) => {
                self.state.devices_config = cfg;
            }
            StateAction::NextTab => {
                self.state.active_tab = self.state.active_tab.next();
            }
            StateAction::PrevTab => {
                self.state.active_tab = self.state.active_tab.prev();
            }
            StateAction::SetSelectedConnection(device) => {
                self.state.app_config.selected_device = Some(device);
            }
            StateAction::UnsetConnection => {
                self.state.app_config.selected_device = None;
            }
            StateAction::SetConnectionState(s) => {
                self.state.connection_state = s;
            }
            StateAction::PushLogRecord(r) => {
                self.state.logs.push(r);
            }
            StateAction::SetDevicesDiscoveringState(s) => {
                self.state.device_discovering_state = s;
            }
            StateAction::SetDiscoveredDevices(devices) => {
                self.state.discovered_devices = devices;
            }
            StateAction::AddTcpDevice(hostaddr) => {
                if !self.state.devices_config.tcp_devices.contains(&hostaddr) {
                    self.state.devices_config.tcp_devices.push(hostaddr);
                }
            }
            StateAction::RemoveTcpDevice(hostaddr) => {
                let maybe_index = self
                    .state
                    .devices_config
                    .tcp_devices
                    .iter()
                    .position(|h| h == &hostaddr);

                if let Some(index) = maybe_index {
                    self.state.devices_config.tcp_devices.remove(index);
                }
            }
        }

        if self.state != prev_state {
            if let Err(e) = self.state_tx.send(self.state.clone()) {
                tracing::error!("failed to send state update: {:?}", e);
            }
        }
    }
}
