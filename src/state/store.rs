use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
    u32, u128,
};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use meshtastic::protobufs::{config, module_config};
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        watch,
    },
    time,
};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::{
    state::{State, StateAction},
    types::{
        Channel, ConnectionState, DeviceDiscoveringState, NodesSortBy, SettingsFormState, Tab,
    },
};

const TICK_INTERVAL_MILLIS: u64 = 33;
const RX_TIMEOUT_MILLIS: u128 = 250;
const TOAST_QUICK_TIMEOUT_MILLIS: u128 = 500;
const SPLASH_LOGO_TIMEOUT_MILLIS: u128 = 1000;

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
        let mut tick_interval = time::interval(Duration::from_millis(TICK_INTERVAL_MILLIS));

        loop {
            tokio::select! {
                Some(action) = self.action_rx.recv() => self.handle_action(action)?,
                _ = tick_interval.tick() => self.handle_tick()?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_action(&mut self, action: StateAction) -> anyhow::Result<()> {
        let mut is_changed = false;

        match action {
            StateAction::SplashLogo => {
                self.state.splash_logo = true;
                self.state.splash_logo_t = Instant::now();
                is_changed = true;
            }
            StateAction::AppConfigApply(cfg) => {
                self.state.active_tab = cfg.active_tab;
                self.state.active_device = cfg.active_device;
                self.state.tcp_devices = cfg.tcp_devices;
                self.state.nodes_sort_by = cfg.nodes_sort_by;
                is_changed = true;
            }
            StateAction::TabSwitchTo(tab) => {
                self.state.active_tab = tab;
                self.state.need_clear_frame = true;
                is_changed = true;
            }
            StateAction::TabSwitchToNext => {
                self.state.active_tab = self.state.active_tab.next();
                self.state.need_clear_frame = true;
                is_changed = true;
            }
            StateAction::TabSwitchToPrevious => {
                self.state.active_tab = self.state.active_tab.prev();
                self.state.need_clear_frame = true;
                is_changed = true;
            }
            StateAction::DeviceActiveSet(device) => {
                self.state.active_device = Some(device);
                is_changed = true;
            }
            StateAction::ConnectionStart => {
                self.state.connection_state = ConnectionState::Connecting;
                self.state.connection_attempt += 1;
                self.state.reconnection_backoff = None;
                is_changed = true;

                tracing::debug!("connection attempt #{}", self.state.connection_attempt);
            }
            StateAction::ConnectionFail(error) => {
                self.state.connection_state = ConnectionState::ProblemDetected {
                    since: Instant::now(),
                    error,
                };
                is_changed = true;
            }
            StateAction::ConnectionStop => {
                self.state.connection_state = ConnectionState::NotConnected;
                self.state.connection_attempt = 0;
                self.state.reconnection_backoff = None;
                self.state.active_device = None;
                self.state.channels.clear();
                self.state.nodes_sort.clear();
                self.state.nodes.clear();
                self.state.online_nodes = 0;
                is_changed = true;
            }
            StateAction::ConnectionSuccess => {
                self.state.connection_state = ConnectionState::Connected;
                self.state.connection_attempt = 0;
                self.state.reconnection_backoff = None;
                is_changed = true;
            }
            StateAction::ReconnectionBackoffSet(duration) => {
                self.state.reconnection_backoff = Some(duration);
                is_changed = true;
            }
            StateAction::LogRecordAdd(r) => {
                self.state.logs.push(r);
                is_changed = true;
            }
            StateAction::DeviceDiscoveringStart => {
                self.state.device_discovering_state = DeviceDiscoveringState::Discovering;
                is_changed = true;
            }
            StateAction::DeviceDiscoveringFail(error) => {
                self.state.device_discovering_state = DeviceDiscoveringState::Failed(error);
                is_changed = true;
            }
            StateAction::DeviceDiscoveringDone(devices) => {
                self.state.discovered_devices = devices;
                self.state.device_discovering_state = DeviceDiscoveringState::Done;
                is_changed = true;
            }
            StateAction::DevicesAddTcp(hostaddr) => {
                if !self.state.tcp_devices.contains(&hostaddr) {
                    self.state.tcp_devices.push(hostaddr);
                    is_changed = true;
                }
            }
            StateAction::DeviceConfigSet(variant) => {
                match variant {
                    config::PayloadVariant::Bluetooth(cfg) => {
                        self.state.device_config.bluetooth = Some(cfg);
                    }
                    config::PayloadVariant::Device(cfg) => {
                        self.state.device_config.device = Some(cfg);
                    }
                    config::PayloadVariant::DeviceUi(cfg) => {
                        self.state.device_config.device_ui = Some(cfg);
                    }
                    config::PayloadVariant::Display(cfg) => {
                        self.state.device_config.display = Some(cfg);
                    }
                    config::PayloadVariant::Lora(cfg) => {
                        self.state.device_config.lora = Some(cfg);
                    }
                    config::PayloadVariant::Network(cfg) => {
                        self.state.device_config.network = Some(cfg);
                    }
                    config::PayloadVariant::Position(cfg) => {
                        self.state.device_config.position = Some(cfg);
                    }
                    config::PayloadVariant::Power(cfg) => {
                        self.state.device_config.power = Some(cfg);
                    }
                    config::PayloadVariant::Security(cfg) => {
                        self.state.device_config.security = Some(cfg);
                    }
                    config::PayloadVariant::Sessionkey(cfg) => {
                        self.state.device_config.sessionkey = Some(cfg);
                    }
                }

                is_changed = true;
            }
            StateAction::DeviceModuleConfigSet(variant) => {
                match variant {
                    module_config::PayloadVariant::AmbientLighting(cfg) => {
                        self.state.device_module_config.ambient_lighting = Some(cfg);
                    }
                    module_config::PayloadVariant::Audio(cfg) => {
                        self.state.device_module_config.audio = Some(cfg);
                    }
                    module_config::PayloadVariant::CannedMessage(cfg) => {
                        self.state.device_module_config.canned_message = Some(cfg);
                    }
                    module_config::PayloadVariant::DetectionSensor(cfg) => {
                        self.state.device_module_config.detection_sensor = Some(cfg);
                    }
                    module_config::PayloadVariant::ExternalNotification(cfg) => {
                        self.state.device_module_config.external_notification = Some(cfg);
                    }
                    module_config::PayloadVariant::Mqtt(cfg) => {
                        self.state.device_module_config.mqtt = Some(cfg);
                    }
                    module_config::PayloadVariant::NeighborInfo(cfg) => {
                        self.state.device_module_config.neighbor = Some(cfg);
                    }
                    module_config::PayloadVariant::Paxcounter(cfg) => {
                        self.state.device_module_config.paxcounter = Some(cfg);
                    }
                    module_config::PayloadVariant::RangeTest(cfg) => {
                        self.state.device_module_config.range_test = Some(cfg);
                    }
                    module_config::PayloadVariant::RemoteHardware(cfg) => {
                        self.state.device_module_config.remote_hardware = Some(cfg);
                    }
                    module_config::PayloadVariant::Serial(cfg) => {
                        self.state.device_module_config.serial = Some(cfg);
                    }
                    module_config::PayloadVariant::Statusmessage(cfg) => {
                        self.state.device_module_config.status_message = Some(cfg);
                    }
                    module_config::PayloadVariant::StoreForward(cfg) => {
                        self.state.device_module_config.store_forward = Some(cfg);
                    }
                    module_config::PayloadVariant::Telemetry(cfg) => {
                        self.state.device_module_config.telemetry = Some(cfg);
                    }
                    module_config::PayloadVariant::TrafficManagement(cfg) => {
                        self.state.device_module_config.traffic_management = Some(cfg);
                    }
                }

                is_changed = true;
            }
            StateAction::DevicesRemoveTcp(hostaddr) => {
                self.state
                    .tcp_devices
                    .iter()
                    .position(|h| h == &hostaddr)
                    .map(|index| {
                        self.state.tcp_devices.remove(index);
                        is_changed = true;
                    });
            }
            StateAction::NodeAdd(mut node) => {
                if let Some(number) = self.state.my_node_key
                    && node.key == number
                {
                    node.my = true;
                }

                self.state.nodes.insert(node.key, node);

                self.update_nodes_sort();
                is_changed = true;
            }
            StateAction::ChannelEnsure(key, channel) => {
                self.state.channels.entry(key).or_insert(channel);
                is_changed = true;
            }
            StateAction::ChannelActiveSet(id) => {
                self.state.active_channel_key = Some(id);
                is_changed = true;
            }
            StateAction::ChannelActiveUnset => {
                self.state.active_channel_key = None;
                is_changed = true;
            }
            StateAction::RxTrigger => {
                self.state.rx_t = Instant::now();
                self.state.rx = true;
                is_changed = true;
            }
            StateAction::NodesSortBySet(sort_by) => {
                self.state.nodes_sort_by = sort_by;
                self.update_nodes_sort();
                is_changed = true;
            }
            StateAction::NodesOnlineSet(count) => {
                self.state.online_nodes = count;
                is_changed = true;
            }
            StateAction::NodeUpdateLastHeard {
                node_key,
                hops,
                snr,
            } => {
                if let Some(node) = self.state.nodes.get_mut(&node_key) {
                    node.last_heard = Some(Utc::now());
                    node.hops_away = Some(hops);

                    if hops == 0 {
                        node.snr = snr;
                    }

                    self.update_nodes_sort();
                    is_changed = true;
                }
            }
            StateAction::MyNodeKeySet(number) => {
                self.state.my_node_key = Some(number);

                if let Some(node) = self.state.nodes.get_mut(&number) {
                    node.my = true;
                    is_changed = true;
                }
            }
            StateAction::DirectChatStart(node_key) => {
                self.state
                    .channels
                    .entry(node_key)
                    .or_insert(Channel::direct(node_key));

                self.state.active_channel_key = Some(node_key);
                self.state.active_tab = Tab::Chat;
                is_changed = true;
            }
            StateAction::MessageAdd(channel_key, message) => {
                if let Some(messages_vec) = self.state.messages.get_mut(&channel_key) {
                    messages_vec.push_back(message);
                } else {
                    self.state
                        .messages
                        .insert(channel_key, VecDeque::from(vec![message]));
                }

                is_changed = true;
            }
            StateAction::MessageReactionAdd {
                channel_key,
                message_id,
                emoji,
                node_key,
            } => {
                if let Some(message) = self
                    .state
                    .messages
                    .get_mut(&channel_key)
                    .and_then(|messages| messages.iter_mut().find(|msg| msg.id == message_id))
                {
                    message
                        .reactions
                        .entry(emoji)
                        .or_insert_with(HashMap::new)
                        .insert(node_key, Utc::now());
                }

                is_changed = true;
            }
            StateAction::MessageAck(channel_key, message_id) => {
                if let Some(message) = self
                    .state
                    .messages
                    .get_mut(&channel_key)
                    .and_then(|messages| messages.iter_mut().find(|msg| msg.id == message_id))
                {
                    message.acked = true;
                }

                is_changed = true;
            }
            StateAction::FrameCleared => {
                self.state.need_clear_frame = false;
                is_changed = true;
            }
            StateAction::Toast(toast) => {
                self.state.toast_queue.push_back(toast);
                is_changed = true;
            }
            StateAction::SettingsFormLoadingStart { id } => {
                self.state.settings_form_original_data = None;
                self.state.settings_form_data = None;
                self.state.settings_form_is_changed = false;
                self.state.settings_form_state = SettingsFormState::Loading { id };
                is_changed = true;
            }
            StateAction::SettingsFormLoadingFail { id, error } => {
                self.state.settings_form_original_data = None;
                self.state.settings_form_data = None;
                self.state.settings_form_is_changed = false;
                self.state.settings_form_state = SettingsFormState::LoadingFailed { id, error };
                is_changed = true;
            }
            StateAction::SettingsFormLoadingDone { id, data } => {
                self.state.settings_form_original_data = Some(data.clone());
                self.state.settings_form_data = Some(data);
                self.state.settings_form_is_changed = false;
                self.state.settings_form_state = SettingsFormState::Loaded { id };
                is_changed = true;
            }
            StateAction::SettingsFormClose => {
                self.state.settings_form_original_data = None;
                self.state.settings_form_data = None;
                self.state.settings_form_is_changed = false;
                self.state.settings_form_state = SettingsFormState::Inactive;
                is_changed = true;
            }
            StateAction::SettingsFormReset => {
                self.state.settings_form_data = self.state.settings_form_original_data.clone();
                self.state.settings_form_is_changed = false;
                is_changed = true;
            }
            StateAction::SettingsFormValueSet { key, value } => {
                if let Some(data) = self.state.settings_form_data.as_mut() {
                    data.insert(key, value);
                    self.state.settings_form_is_changed =
                        self.state.settings_form_data != self.state.settings_form_original_data;

                    is_changed = true;
                }
            }
        }

        if is_changed {
            self.state_tx.send(self.state.clone())?;
        }

        Ok(())
    }

    fn handle_tick(&mut self) -> anyhow::Result<()> {
        if self.state.rx && self.state.rx_t.elapsed().as_millis() > RX_TIMEOUT_MILLIS {
            self.state.rx = false;
            self.state_tx.send(self.state.clone())?;
        }

        if self.state.splash_logo
            && self.state.splash_logo_t.elapsed().as_millis() > SPLASH_LOGO_TIMEOUT_MILLIS
        {
            self.state.splash_logo = false;
            self.state_tx.send(self.state.clone())?;
        }

        if let Some(toast) = &self.state.toast {
            // skip toast quickly if there is another in queue
            let timeout = toast
                .kind
                .timeout()
                .min(if self.state.toast_queue.is_empty() {
                    u128::MAX
                } else {
                    TOAST_QUICK_TIMEOUT_MILLIS
                });

            if self.state.toast_t.elapsed().as_millis() > timeout {
                self.state.toast = None;
                self.state_tx.send(self.state.clone())?;
            }
        }

        if !self.state.toast_queue.is_empty() {
            self.state.toast = self.state.toast_queue.pop_front();
            self.state.toast_t = Instant::now();
            self.state_tx.send(self.state.clone())?;
        }

        Ok(())
    }

    fn update_nodes_sort(&mut self) {
        self.state.nodes_sort = self
            .state
            .nodes
            .values()
            .sorted_by(|n1, n2| {
                match (n1.my, n2.my) {
                    (true, true) => return Ordering::Equal,
                    (false, true) => return Ordering::Greater,
                    (true, false) => return Ordering::Less,
                    _ => {}
                };

                match &self.state.nodes_sort_by {
                    NodesSortBy::Hops => n1
                        .hops_away
                        .unwrap_or(u32::MAX)
                        .cmp(&n2.hops_away.unwrap_or(u32::MAX))
                        .then(n1.snr.total_cmp(&n2.snr).reverse()),
                    NodesSortBy::LastHeard => n1
                        .last_heard
                        .unwrap_or(DateTime::default())
                        .cmp(&n2.last_heard.unwrap_or(DateTime::default()))
                        .reverse(),
                    NodesSortBy::ShortName => n1.short_name.cmp(&n2.short_name),
                    NodesSortBy::LongName => n1.long_name.cmp(&n2.long_name),
                    NodesSortBy::HwModel => n1
                        .hw_model
                        .cmp(&n2.hw_model)
                        .then(n1.short_name.cmp(&n2.short_name)),
                    NodesSortBy::Role => n1.role.cmp(&n2.role).then(
                        n1.hops_away
                            .unwrap_or(u32::MAX)
                            .cmp(&n2.hops_away.unwrap_or(u32::MAX))
                            .then(n1.snr.total_cmp(&n2.snr).reverse()),
                    ),
                }
            })
            .map(|node| node.key)
            .collect();
    }
}
