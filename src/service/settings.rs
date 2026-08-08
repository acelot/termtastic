use itertools::Itertools;
use meshtastic::Message;
use meshtastic::protobufs::config::{
    self, BluetoothConfig, DeviceConfig, DisplayConfig, LoRaConfig, NetworkConfig, PositionConfig, PowerConfig,
    SecurityConfig,
};
use meshtastic::protobufs::module_config::{
    AmbientLightingConfig, CannedMessageConfig, DetectionSensorConfig, ExternalNotificationConfig, MqttConfig,
    NeighborInfoConfig, RangeTestConfig, SerialConfig, StoreForwardConfig, TelemetryConfig, TrafficManagementConfig,
};
use meshtastic::protobufs::{
    AdminMessage, Channel as MeshtasticChannel, Config, ModuleConfig, PortNum, admin_message, from_radio, mesh_packet,
    module_config,
};
use nameof::name_of;
use ordermap::OrderMap;
use std::sync::LazyLock;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::serde::{from_formdata, to_formdata};
use crate::types::{
    AppEvent, Channel, FormData, FormId, FormValue, NodeUser, SettingsFormState, SettingsItem, Toast, UiConfig,
};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
};

pub static SETTINGS: LazyLock<Vec<SettingsItem>> = LazyLock::new(|| build_settings());

pub struct SettingsService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    state_changed_rx: broadcast::Receiver<Vec<&'static str>>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl SettingsService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        state_changed_rx: broadcast::Receiver<Vec<&'static str>>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            app_event_rx,
            state_rx,
            state_action_tx,
            state_changed_rx,
            meshtastic_command_tx,
            meshtastic_event_rx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.state_changed_rx.recv() => self.handle_state_change(event)?,
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

    fn handle_state_change(&self, event: Result<Vec<&'static str>, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(changed) => {
                let state = self.state_rx.borrow();

                if changed.contains(&name_of!(ui_config in State))
                    && matches!(
                        state.settings_form_state,
                        SettingsFormState::Saving { id: FormId::AppUi }
                    )
                {
                    self.state_action_tx
                        .send(StateAction::Toast(Toast::success("setting saved")))?;

                    self.start_config_loading(&FormId::AppUi)?;
                }

                if changed.contains(&name_of!(my_node_user_hash in State))
                    && matches!(
                        state.settings_form_state,
                        SettingsFormState::Loaded { id: FormId::DeviceUser }
                    )
                {
                    self.start_config_loading(&FormId::DeviceUser)?;
                }

                if changed.contains(&name_of!(device_canned_messages in State))
                    && matches!(
                        state.settings_form_state,
                        SettingsFormState::Loaded {
                            id: FormId::ModuleCannedMessage
                        }
                    )
                {
                    self.start_config_loading(&FormId::ModuleCannedMessage)?;
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_app_event(&self, event: Result<AppEvent, broadcast::error::RecvError>) -> anyhow::Result<()> {
        match event {
            Ok(app_event) => match app_event {
                AppEvent::SettingsFormSelected(id) => {
                    self.start_config_loading(&id)?;
                }
                AppEvent::SettingsFormCancelRequested => {
                    self.state_action_tx.send(StateAction::SettingsFormClose)?;
                }
                AppEvent::SettingsFormResetRequested => {
                    self.state_action_tx.send(StateAction::SettingsFormReset)?;

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::normal("the data was reset")))?;
                }
                AppEvent::SettingsFormSaveRequested(form_id) => {
                    self.state_action_tx
                        .send(StateAction::Toast(Toast::normal("saving...")))?;

                    self.save_config(&form_id)?;
                }
                AppEvent::SettingsFormItemSubmitted(form_item, value) => {
                    self.state_action_tx.send(StateAction::SettingsFormValueSet {
                        key: form_item.key.clone(),
                        value,
                    })?;
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

    fn handle_meshtastic_event(
        &mut self,
        event: Result<MeshtasticEvent, broadcast::error::RecvError>,
    ) -> anyhow::Result<()> {
        match event {
            Ok(meshtastic_event) => match meshtastic_event {
                MeshtasticEvent::IncomingPacket(packet) => {
                    self.handle_meshtastic_packet(*packet)?;
                }
                MeshtasticEvent::ConfigSaveFailed(form_id)
                | MeshtasticEvent::ChannelsSaveFailed(form_id)
                | MeshtasticEvent::UserSaveFailed(form_id) => {
                    self.state_action_tx
                        .send(StateAction::Toast(Toast::error("save failed (see logs)")))?;

                    self.state_action_tx
                        .send(StateAction::SettingsFormSavingFailed { id: form_id })?;
                }
                MeshtasticEvent::ConfigSaved(form_id) | MeshtasticEvent::UserSaved(form_id) => {
                    self.state_action_tx
                        .send(StateAction::Toast(Toast::success("setting saved")))?;

                    self.start_config_loading(&form_id)?;
                }
                MeshtasticEvent::ChannelsSaved(form_id) => {
                    self.state_action_tx.send(StateAction::ActiveChatUnset)?;

                    self.state_action_tx
                        .send(StateAction::Toast(Toast::success("channels saved")))?;

                    self.start_config_loading(&form_id)?;
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

    fn handle_meshtastic_packet(&mut self, packet: from_radio::PayloadVariant) -> anyhow::Result<()> {
        match packet {
            from_radio::PayloadVariant::Config(Config {
                payload_variant: Some(variant),
            }) => {
                self.state_action_tx.send(StateAction::DeviceConfigSet(variant))?;
            }
            from_radio::PayloadVariant::ModuleConfig(ModuleConfig {
                payload_variant: Some(variant),
            }) => {
                self.state_action_tx.send(StateAction::DeviceModuleConfigSet(variant))?;
            }
            from_radio::PayloadVariant::Packet(mesh_packet) => match &mesh_packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::AdminApp => match AdminMessage::decode(&*data.payload) {
                        Ok(admin_message) => match admin_message.payload_variant {
                            Some(admin_message::PayloadVariant::SetConfig(Config {
                                payload_variant: Some(variant),
                            })) => {
                                self.state_action_tx.send(StateAction::DeviceConfigSet(variant))?;
                            }
                            Some(admin_message::PayloadVariant::SetModuleConfig(ModuleConfig {
                                payload_variant: Some(variant),
                            })) => {
                                self.state_action_tx.send(StateAction::DeviceModuleConfigSet(variant))?;
                            }
                            Some(admin_message::PayloadVariant::SetChannel(channel)) => {
                                self.state_action_tx
                                    .send(StateAction::ChannelSet(channel.index as u32, (&channel).into()))?;
                            }
                            Some(admin_message::PayloadVariant::RebootSeconds(secs)) => {
                                self.state_action_tx.send(StateAction::Toast(Toast::warning(format!(
                                    "device will be rebooted in {} secs...",
                                    secs
                                ))))?;
                            }
                            Some(admin_message::PayloadVariant::ShutdownSeconds(secs)) => {
                                self.state_action_tx.send(StateAction::Toast(Toast::warning(format!(
                                    "device will shutdown in {} secs...",
                                    secs
                                ))))?;
                            }
                            Some(admin_message::PayloadVariant::GetCannedMessageModuleMessagesResponse(messages)) => {
                                self.state_action_tx
                                    .send(StateAction::DeviceCannedMessagesSet(messages))?;
                            }
                            _ => {}
                        },
                        Err(e) => {
                            tracing::debug!("can't decode AdminMessage payload: {:?}", e);
                        }
                    },
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn start_config_loading(&self, id: &FormId) -> anyhow::Result<()> {
        self.state_action_tx
            .send(StateAction::SettingsFormLoadingStart { id: id.clone() })?;

        match self.load_config(&id) {
            Ok(data) => self
                .state_action_tx
                .send(StateAction::SettingsFormLoadingDone { id: id.clone(), data })?,
            Err(e) => self.state_action_tx.send(StateAction::SettingsFormLoadingFail {
                id: id.clone(),
                error: e.to_string(),
            })?,
        }

        Ok(())
    }

    fn load_config(&self, id: &FormId) -> anyhow::Result<FormData> {
        let state = &self.state_rx.borrow();

        let data = match id {
            FormId::AppUi => to_formdata(&state.ui_config)?,
            FormId::RadioLora => to_formdata(
                state
                    .device_config
                    .lora
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Lora config not loaded"))?,
            )?,
            FormId::RadioChannels => {
                if state.channels.len() < 8 {
                    return Err(anyhow::anyhow!("Channels config not loaded (too few channels)"));
                }

                to_formdata(&state.channels)?
            }
            FormId::RadioSecurity => to_formdata(
                state
                    .device_config
                    .security
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Security config not loaded"))?,
            )?,
            FormId::DeviceDevice => to_formdata(
                state
                    .device_config
                    .device
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Device config not loaded"))?,
            )?,
            FormId::DeviceUser => to_formdata(
                state
                    .get_my_node()
                    .and_then(|n| n.user.as_ref())
                    .ok_or(anyhow::anyhow!("User config not loaded"))?,
            )?,
            FormId::DevicePosition => to_formdata(
                state
                    .device_config
                    .position
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Position config not loaded"))?,
            )?,
            FormId::DevicePower => to_formdata(
                state
                    .device_config
                    .power
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Power config not loaded"))?,
            )?,
            FormId::DeviceDisplay => to_formdata(
                state
                    .device_config
                    .display
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Display config not loaded"))?,
            )?,
            FormId::DeviceBluetooth => to_formdata(
                state
                    .device_config
                    .bluetooth
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Bluetooth config not loaded"))?,
            )?,
            FormId::DeviceNetwork => to_formdata(
                state
                    .device_config
                    .network
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Network config not loaded"))?,
            )?,
            FormId::DeviceAdministration => FormData::new(),
            FormId::ModuleMqtt => to_formdata(
                state
                    .device_module_config
                    .mqtt
                    .as_ref()
                    .ok_or(anyhow::anyhow!("MQTT config not loaded"))?,
            )?,
            FormId::ModuleSerial => to_formdata(
                state
                    .device_module_config
                    .serial
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Serial config not loaded"))?,
            )?,
            FormId::ModuleExternalNotification => to_formdata(
                state
                    .device_module_config
                    .external_notification
                    .as_ref()
                    .ok_or(anyhow::anyhow!("External Notification config not loaded"))?,
            )?,
            FormId::ModuleStoreAndForward => to_formdata(
                state
                    .device_module_config
                    .store_forward
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Store & Forward config not loaded"))?,
            )?,
            FormId::ModuleRangeTest => to_formdata(
                state
                    .device_module_config
                    .range_test
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Range Test config not loaded"))?,
            )?,
            FormId::ModuleTelemetry => to_formdata(
                state
                    .device_module_config
                    .telemetry
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Telemetry config not loaded"))?,
            )?,
            FormId::ModuleCannedMessage => {
                let mut form_data = to_formdata(
                    state
                        .device_module_config
                        .canned_message
                        .as_ref()
                        .ok_or(anyhow::anyhow!("Canned Message config not loaded"))?,
                )?;

                form_data.insert(
                    "messages".to_owned(),
                    FormValue::String(state.device_canned_messages.as_ref().unwrap_or(&String::new()).clone()),
                );

                form_data
            }
            FormId::ModuleNeighborInfo => to_formdata(
                state
                    .device_module_config
                    .neighbor
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Neighbor Info config not loaded"))?,
            )?,
            FormId::ModuleAmbientLighting => to_formdata(
                state
                    .device_module_config
                    .ambient_lighting
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Ambient Lighting config not loaded"))?,
            )?,
            FormId::ModuleDetectionSensor => to_formdata(
                state
                    .device_module_config
                    .detection_sensor
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Detection Sensor config not loaded"))?,
            )?,
            FormId::ModuleTrafficManagement => to_formdata(
                state
                    .device_module_config
                    .traffic_management
                    .as_ref()
                    .ok_or(anyhow::anyhow!(
                        "Traffic Management config not loaded or it's not supported by the device firmware"
                    ))?,
            )?,
        };

        Ok(data)
    }

    fn save_config(&self, id: &FormId) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();
        let form_data = state.settings_form_data.as_ref().expect("should be Some");

        self.state_action_tx
            .send(StateAction::SettingsFormSavingStart { id: id.clone() })?;

        match id {
            FormId::AppUi => {
                self.state_action_tx.send(StateAction::UiConfigSet {
                    config: from_formdata::<UiConfig>(&form_data)?,
                })?;
            }
            FormId::RadioLora => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Lora(from_formdata::<LoRaConfig>(&form_data)?),
                })?;
            }
            FormId::RadioChannels => {
                let channels = from_formdata::<OrderMap<String, Channel>>(&form_data)?;
                let msh_channels = channels
                    .iter()
                    .map(|(_, ch)| Into::<MeshtasticChannel>::into(ch))
                    .sorted_by_key(|ch| ch.index)
                    .collect();

                self.meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveChannelsConfig {
                        form_id: id.clone(),
                        my_node_num: state.my_node_key.expect("should be Some"),
                        channels: msh_channels,
                    })?;
            }
            FormId::RadioSecurity => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Security(from_formdata::<SecurityConfig>(&form_data)?),
                })?;
            }
            FormId::DeviceDevice => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Device(from_formdata::<DeviceConfig>(&form_data)?),
                })?;
            }
            FormId::DeviceUser => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveUser {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    user: from_formdata::<NodeUser>(&form_data)?.into(),
                })?;
            }
            FormId::DevicePosition => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Position(from_formdata::<PositionConfig>(&form_data)?),
                })?;
            }
            FormId::DevicePower => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Power(from_formdata::<PowerConfig>(&form_data)?),
                })?;
            }
            FormId::DeviceDisplay => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Display(from_formdata::<DisplayConfig>(&form_data)?),
                })?;
            }
            FormId::DeviceBluetooth => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Bluetooth(from_formdata::<BluetoothConfig>(&form_data)?),
                })?;
            }
            FormId::DeviceNetwork => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Network(from_formdata::<NetworkConfig>(&form_data)?),
                })?;
            }
            FormId::DeviceAdministration => {}
            FormId::ModuleMqtt => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Mqtt(from_formdata::<MqttConfig>(&form_data)?),
                })?;
            }
            FormId::ModuleSerial => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Serial(from_formdata::<SerialConfig>(&form_data)?),
                })?;
            }
            FormId::ModuleExternalNotification => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::ExternalNotification(from_formdata::<
                        ExternalNotificationConfig,
                    >(&form_data)?),
                })?;
            }
            FormId::ModuleStoreAndForward => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::StoreForward(from_formdata::<StoreForwardConfig>(
                        &form_data,
                    )?),
                })?;
            }
            FormId::ModuleRangeTest => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::RangeTest(from_formdata::<RangeTestConfig>(&form_data)?),
                })?;
            }
            FormId::ModuleTelemetry => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Telemetry(from_formdata::<TelemetryConfig>(&form_data)?),
                })?;
            }
            FormId::ModuleCannedMessage => {
                let mut cloned_form_data = form_data.clone();
                let messages = cloned_form_data
                    .remove("messages")
                    .expect("should be Some")
                    .as_string()
                    .clone();

                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::CannedMessage(from_formdata::<CannedMessageConfig>(
                        &cloned_form_data,
                    )?),
                })?;

                self.meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveCannedMessages {
                        my_node_num: state.my_node_key.expect("should be Some"),
                        messages,
                    })?;
            }
            FormId::ModuleNeighborInfo => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::NeighborInfo(from_formdata::<NeighborInfoConfig>(
                        &form_data,
                    )?),
                })?;
            }
            FormId::ModuleAmbientLighting => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::AmbientLighting(from_formdata::<AmbientLightingConfig>(
                        &form_data,
                    )?),
                })?;
            }
            FormId::ModuleDetectionSensor => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::DetectionSensor(from_formdata::<DetectionSensorConfig>(
                        &form_data,
                    )?),
                })?;
            }
            FormId::ModuleTrafficManagement => {
                self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id: id.clone(),
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::TrafficManagement(from_formdata::<TrafficManagementConfig>(
                        &form_data,
                    )?),
                })?;
            }
        };

        Ok(())
    }
}

fn build_settings() -> Vec<SettingsItem> {
    Vec::from([
        // App
        SettingsItem::group("App"),
        SettingsItem::form("UI", FormId::AppUi),
        // Radio
        SettingsItem::group("Radio"),
        SettingsItem::form("LoRa", FormId::RadioLora),
        SettingsItem::form("Channels", FormId::RadioChannels),
        SettingsItem::form("Security", FormId::RadioSecurity),
        // Device
        SettingsItem::group("Device"),
        SettingsItem::form("User", FormId::DeviceUser),
        SettingsItem::form("Device", FormId::DeviceDevice),
        SettingsItem::form("Position", FormId::DevicePosition),
        SettingsItem::form("Power", FormId::DevicePower),
        SettingsItem::form("Display", FormId::DeviceDisplay),
        SettingsItem::form("Bluetooth", FormId::DeviceBluetooth),
        SettingsItem::form("Network", FormId::DeviceNetwork),
        SettingsItem::form("Administration", FormId::DeviceAdministration),
        // Module
        SettingsItem::group("Module"),
        SettingsItem::form("MQTT", FormId::ModuleMqtt),
        SettingsItem::form("Serial", FormId::ModuleSerial),
        SettingsItem::form("External Notification", FormId::ModuleExternalNotification),
        SettingsItem::form("Store & Forward", FormId::ModuleStoreAndForward),
        SettingsItem::form("Range Test", FormId::ModuleRangeTest),
        SettingsItem::form("Telemetry", FormId::ModuleTelemetry),
        SettingsItem::form("Canned Message", FormId::ModuleCannedMessage),
        SettingsItem::form("Neighbor Info", FormId::ModuleNeighborInfo),
        SettingsItem::form("Ambient Lighting", FormId::ModuleAmbientLighting),
        SettingsItem::form("Detection Sensor", FormId::ModuleDetectionSensor),
        SettingsItem::form("Traffic Management", FormId::ModuleTrafficManagement),
    ])
}
