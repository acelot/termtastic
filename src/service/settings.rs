use itertools::Itertools;
use meshtastic::Message;
use meshtastic::protobufs::config::{
    self, BluetoothConfig, DeviceConfig, DisplayConfig, LoRaConfig, NetworkConfig, PositionConfig, PowerConfig,
    SecurityConfig,
};
use meshtastic::protobufs::module_config::{
    AmbientLightingConfig, CannedMessageConfig, DetectionSensorConfig, ExternalNotificationConfig, MeshBeaconConfig,
    MqttConfig, NeighborInfoConfig, RangeTestConfig, SerialConfig, StoreForwardConfig, TakConfig, TelemetryConfig,
    TrafficManagementConfig,
};
use meshtastic::protobufs::{
    AdminMessage, Channel as MeshtasticChannel, Config, ModuleConfig, PortNum, admin_message, from_radio, mesh_packet,
    module_config,
};
use ordermap::OrderMap;
use std::sync::LazyLock;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::serde::{from_formdata, to_formdata};
use crate::service::FW_VERSION_GT_2_8;
use crate::types::{
    AppGeneralConfig, AppUiConfig, Channel, ConnectionState, FormData, FormId, FormValue, NodeUser, SettingsFormState,
    SettingsItem, Toast, UiEvent,
};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
};

static VERSION_REQ_GT_2_8: LazyLock<semver::VersionReq> = LazyLock::new(|| semver::VersionReq::parse("^2.8").unwrap());

pub struct SettingsService {
    ui_event_rx: broadcast::Receiver<UiEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    stored_action_rx: broadcast::Receiver<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl SettingsService {
    pub fn new(
        ui_event_rx: broadcast::Receiver<UiEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        stored_action_rx: broadcast::Receiver<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
            ui_event_rx,
            state_rx,
            state_action_tx,
            stored_action_rx,
            meshtastic_command_tx,
            meshtastic_event_rx,
        }
    }

    pub async fn run(mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                event = self.stored_action_rx.recv() => self.handle_stored_action(event),
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

    fn handle_stored_action(&self, event: Result<StateAction, broadcast::error::RecvError>) {
        let state = self.state_rx.borrow();

        match event {
            Ok(StateAction::AppUiConfigSet(_))
                if matches!(
                    state.settings_form_state,
                    SettingsFormState::Saving { id: FormId::AppUi }
                ) =>
            {
                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::success("setting saved")));

                self.start_config_loading(FormId::AppUi);
            }
            Ok(StateAction::AppGeneralConfigSet(_))
                if matches!(
                    state.settings_form_state,
                    SettingsFormState::Saving { id: FormId::AppGeneral }
                ) =>
            {
                let _ = self
                    .state_action_tx
                    .send(StateAction::Toast(Toast::success("setting saved")));

                self.start_config_loading(FormId::AppGeneral);
            }
            Ok(StateAction::NodeOwnerSet(_))
                if matches!(
                    state.settings_form_state,
                    SettingsFormState::Saving { id: FormId::DeviceUser }
                ) =>
            {
                self.start_config_loading(FormId::DeviceUser);
            }
            Ok(StateAction::DeviceCannedMessagesSet { .. })
                if matches!(
                    state.settings_form_state,
                    SettingsFormState::Saving {
                        id: FormId::ModuleCannedMessage
                    }
                ) =>
            {
                self.start_config_loading(FormId::ModuleCannedMessage);
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }

    fn handle_ui_event(&self, event: Result<UiEvent, broadcast::error::RecvError>) {
        match event {
            Ok(app_event) => match app_event {
                UiEvent::SettingsFormSelected(id) => {
                    self.start_config_loading(id);
                }
                UiEvent::SettingsFormCancelRequested => {
                    let _ = self.state_action_tx.send(StateAction::SettingsFormClose);
                }
                UiEvent::SettingsFormResetRequested => {
                    let _ = self.state_action_tx.send(StateAction::SettingsFormReset);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("the data was reset")));
                }
                UiEvent::SettingsFormSaveRequested(form_id) => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::normal("saving...")));

                    if let Err(e) = self.save_config(form_id) {
                        tracing::error!("settings save failed: {}", e);

                        let _ = self
                            .state_action_tx
                            .send(StateAction::Toast(Toast::error("save failed (see logs)")));

                        let _ = self
                            .state_action_tx
                            .send(StateAction::SettingsFormSavingFailed { id: form_id });
                    }
                }
                UiEvent::SettingsFormItemSubmitted(form_item, value) => {
                    let _ = self.state_action_tx.send(StateAction::SettingsFormValueSet {
                        key: form_item.key.clone(),
                        value,
                    });
                }
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
                MeshtasticEvent::ConfigSaveFailed(form_id)
                | MeshtasticEvent::ChannelsSaveFailed(form_id)
                | MeshtasticEvent::UserSaveFailed(form_id) => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::error("save failed (see logs)")));

                    let _ = self
                        .state_action_tx
                        .send(StateAction::SettingsFormSavingFailed { id: form_id });
                }
                MeshtasticEvent::ConfigSaved(form_id) | MeshtasticEvent::UserSaved(form_id) => {
                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::success("setting saved")));

                    self.start_config_loading(form_id);
                }
                MeshtasticEvent::ChannelsSaved(form_id) => {
                    let _ = self.state_action_tx.send(StateAction::ActiveChatUnset);

                    let _ = self
                        .state_action_tx
                        .send(StateAction::Toast(Toast::success("channels saved")));

                    self.start_config_loading(form_id);
                }
                _ => {}
            },
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("broadcast receiver lagged by {} events", n);
            }
            _ => {}
        }
    }

    fn handle_meshtastic_packet(&mut self, packet: from_radio::PayloadVariant) {
        match packet {
            from_radio::PayloadVariant::Config(Config {
                payload_variant: Some(variant),
            }) => {
                let _ = self.state_action_tx.send(StateAction::DeviceConfigSet(variant));
            }
            from_radio::PayloadVariant::ModuleConfig(ModuleConfig {
                payload_variant: Some(variant),
            }) => {
                let _ = self.state_action_tx.send(StateAction::DeviceModuleConfigSet(variant));
            }
            from_radio::PayloadVariant::Packet(mesh_packet) => match &mesh_packet.payload_variant {
                Some(mesh_packet::PayloadVariant::Decoded(data)) => match data.portnum() {
                    PortNum::AdminApp => match AdminMessage::decode(&*data.payload) {
                        Ok(admin_message) => match admin_message.payload_variant {
                            Some(admin_message::PayloadVariant::SetConfig(Config {
                                payload_variant: Some(variant),
                            })) => {
                                let _ = self.state_action_tx.send(StateAction::DeviceConfigSet(variant));
                            }
                            Some(admin_message::PayloadVariant::SetModuleConfig(ModuleConfig {
                                payload_variant: Some(variant),
                            })) => {
                                let _ = self.state_action_tx.send(StateAction::DeviceModuleConfigSet(variant));
                            }
                            Some(admin_message::PayloadVariant::SetChannel(channel)) => {
                                let _ = self.state_action_tx.send(StateAction::ChannelSet {
                                    key: channel.index as u32,
                                    channel: (&channel).into(),
                                });
                            }
                            Some(admin_message::PayloadVariant::RebootSeconds(secs)) => {
                                let _ = self.state_action_tx.send(StateAction::Toast(Toast::warning(format!(
                                    "device will be rebooted in {} secs...",
                                    secs
                                ))));
                            }
                            Some(admin_message::PayloadVariant::ShutdownSeconds(secs)) => {
                                let _ = self.state_action_tx.send(StateAction::Toast(Toast::warning(format!(
                                    "device will shutdown in {} secs...",
                                    secs
                                ))));
                            }
                            Some(admin_message::PayloadVariant::GetCannedMessageModuleMessagesResponse(messages)) => {
                                let _ = self
                                    .state_action_tx
                                    .send(StateAction::DeviceCannedMessagesSet { messages });
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
    }

    fn start_config_loading(&self, id: FormId) {
        let _ = self.state_action_tx.send(StateAction::SettingsFormLoadingStart { id });

        match self.load_config(id) {
            Ok(data) => {
                let _ = self
                    .state_action_tx
                    .send(StateAction::SettingsFormLoadingDone { id, data });
            }
            Err(e) => {
                let _ = self.state_action_tx.send(StateAction::SettingsFormLoadingFail {
                    id,
                    error: e.to_string(),
                });
            }
        }
    }

    fn load_config(&self, id: FormId) -> anyhow::Result<FormData> {
        let state = self.state_rx.borrow();

        let mut data = match id {
            FormId::AppGeneral => to_formdata(&state.app_general_config)?,
            FormId::AppUi => to_formdata(&state.app_ui_config)?,
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
            FormId::ModuleTak => to_formdata(state.device_module_config.tak.as_ref().ok_or(anyhow::anyhow!(
                "TAK config not loaded or it's not supported by the device firmware"
            ))?)?,
            FormId::ModuleMeshBeacon => to_formdata(state.device_module_config.mesh_beacon.as_ref().ok_or(
                anyhow::anyhow!("Mesh Beacon config not loaded or it's not supported by the device firmware"),
            )?)?,
        };

        // inject FW version data
        data.insert(
            FW_VERSION_GT_2_8.to_owned(),
            FormValue::Bool(
                state
                    .device_fw_version
                    .as_ref()
                    .map(|v| VERSION_REQ_GT_2_8.matches(v))
                    .unwrap_or(false),
            ),
        );

        Ok(data)
    }

    fn save_config(&self, form_id: FormId) -> anyhow::Result<()> {
        let state = self.state_rx.borrow();
        let form_data = state.settings_form_data.as_ref().expect("should be Some");

        let _ = self
            .state_action_tx
            .send(StateAction::SettingsFormSavingStart { id: form_id });

        match form_id {
            FormId::AppGeneral => {
                let _ = self
                    .state_action_tx
                    .send(StateAction::AppGeneralConfigSet(from_formdata::<AppGeneralConfig>(
                        form_data,
                    )?));
            }
            FormId::AppUi => {
                let _ = self
                    .state_action_tx
                    .send(StateAction::AppUiConfigSet(from_formdata::<AppUiConfig>(form_data)?));
            }
            FormId::RadioLora => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Lora(from_formdata::<LoRaConfig>(form_data)?),
                });
            }
            FormId::RadioChannels => {
                let channels = from_formdata::<OrderMap<String, Channel>>(form_data)?;
                let msh_channels = channels
                    .iter()
                    .map(|(_, ch)| Into::<MeshtasticChannel>::into(ch))
                    .sorted_by_key(|ch| ch.index)
                    .collect();

                let _ = self
                    .meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveChannelsConfig {
                        form_id,
                        my_node_num: state.my_node_key.expect("should be Some"),
                        channels: msh_channels,
                    });
            }
            FormId::RadioSecurity => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Security(from_formdata::<SecurityConfig>(form_data)?),
                });
            }
            FormId::DeviceDevice => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Device(from_formdata::<DeviceConfig>(form_data)?),
                });
            }
            FormId::DeviceUser => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveUser {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    user: from_formdata::<NodeUser>(form_data)?.into(),
                });
            }
            FormId::DevicePosition => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Position(from_formdata::<PositionConfig>(form_data)?),
                });
            }
            FormId::DevicePower => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Power(from_formdata::<PowerConfig>(form_data)?),
                });
            }
            FormId::DeviceDisplay => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Display(from_formdata::<DisplayConfig>(form_data)?),
                });
            }
            FormId::DeviceBluetooth => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Bluetooth(from_formdata::<BluetoothConfig>(form_data)?),
                });
            }
            FormId::DeviceNetwork => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: config::PayloadVariant::Network(from_formdata::<NetworkConfig>(form_data)?),
                });
            }
            FormId::DeviceAdministration => {}
            FormId::ModuleMqtt => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Mqtt(from_formdata::<MqttConfig>(form_data)?),
                });
            }
            FormId::ModuleSerial => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Serial(from_formdata::<SerialConfig>(form_data)?),
                });
            }
            FormId::ModuleExternalNotification => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::ExternalNotification(from_formdata::<
                        ExternalNotificationConfig,
                    >(form_data)?),
                });
            }
            FormId::ModuleStoreAndForward => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::StoreForward(from_formdata::<StoreForwardConfig>(
                        form_data,
                    )?),
                });
            }
            FormId::ModuleRangeTest => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::RangeTest(from_formdata::<RangeTestConfig>(form_data)?),
                });
            }
            FormId::ModuleTelemetry => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Telemetry(from_formdata::<TelemetryConfig>(form_data)?),
                });
            }
            FormId::ModuleCannedMessage => {
                let mut cloned_form_data = form_data.clone();
                let messages = cloned_form_data
                    .remove("messages")
                    .expect("should be Some")
                    .as_string()
                    .clone();

                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::CannedMessage(from_formdata::<CannedMessageConfig>(
                        &cloned_form_data,
                    )?),
                });

                let _ = self
                    .meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveCannedMessages {
                        my_node_num: state.my_node_key.expect("should be Some"),
                        messages,
                    });
            }
            FormId::ModuleNeighborInfo => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::NeighborInfo(from_formdata::<NeighborInfoConfig>(
                        form_data,
                    )?),
                });
            }
            FormId::ModuleAmbientLighting => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::AmbientLighting(from_formdata::<AmbientLightingConfig>(
                        form_data,
                    )?),
                });
            }
            FormId::ModuleDetectionSensor => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::DetectionSensor(from_formdata::<DetectionSensorConfig>(
                        form_data,
                    )?),
                });
            }
            FormId::ModuleTrafficManagement => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::TrafficManagement(from_formdata::<TrafficManagementConfig>(
                        form_data,
                    )?),
                });
            }
            FormId::ModuleTak => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::Tak(from_formdata::<TakConfig>(form_data)?),
                });
            }
            FormId::ModuleMeshBeacon => {
                let _ = self.meshtastic_command_tx.send(CommandToMeshtastic::SaveModuleConfig {
                    form_id,
                    my_node_num: state.my_node_key.expect("should be Some"),
                    config: module_config::PayloadVariant::MeshBeacon(from_formdata::<MeshBeaconConfig>(form_data)?),
                });
            }
        };

        Ok(())
    }
}

pub fn update_settings_menu(state: &mut State) {
    let is_connected = matches!(state.connection_state, ConnectionState::Connected);

    let is_fw_version_gt_2_8 = state
        .device_fw_version
        .as_ref()
        .map(|v| VERSION_REQ_GT_2_8.matches(v))
        .unwrap_or(false);

    state.settings_menu = [
        // App
        Some(SettingsItem::group("App")),
        Some(SettingsItem::form("General", FormId::AppGeneral)),
        Some(SettingsItem::form("UI", FormId::AppUi)),
        // Radio
        is_connected.then_some(SettingsItem::group("Radio")),
        is_connected.then_some(SettingsItem::form("LoRa", FormId::RadioLora)),
        is_connected.then_some(SettingsItem::form("Channels", FormId::RadioChannels)),
        is_connected.then_some(SettingsItem::form("Security", FormId::RadioSecurity)),
        // Device
        is_connected.then_some(SettingsItem::group("Device")),
        is_connected.then_some(SettingsItem::form("User", FormId::DeviceUser)),
        is_connected.then_some(SettingsItem::form("Device", FormId::DeviceDevice)),
        is_connected.then_some(SettingsItem::form("Position", FormId::DevicePosition)),
        is_connected.then_some(SettingsItem::form("Power", FormId::DevicePower)),
        is_connected.then_some(SettingsItem::form("Display", FormId::DeviceDisplay)),
        is_connected.then_some(SettingsItem::form("Bluetooth", FormId::DeviceBluetooth)),
        is_connected.then_some(SettingsItem::form("Network", FormId::DeviceNetwork)),
        is_connected.then_some(SettingsItem::form("Administration", FormId::DeviceAdministration)),
        // Module
        is_connected.then_some(SettingsItem::group("Module")),
        is_connected.then_some(SettingsItem::form("MQTT", FormId::ModuleMqtt)),
        is_connected.then_some(SettingsItem::form("Serial", FormId::ModuleSerial)),
        is_connected.then_some(SettingsItem::form(
            "External Notification",
            FormId::ModuleExternalNotification,
        )),
        is_connected.then_some(SettingsItem::form("Store & Forward", FormId::ModuleStoreAndForward)),
        is_connected.then_some(SettingsItem::form("Range Test", FormId::ModuleRangeTest)),
        is_connected.then_some(SettingsItem::form("Telemetry", FormId::ModuleTelemetry)),
        is_connected.then_some(SettingsItem::form("Canned Message", FormId::ModuleCannedMessage)),
        is_connected.then_some(SettingsItem::form("Neighbor Info", FormId::ModuleNeighborInfo)),
        is_connected.then_some(SettingsItem::form("Ambient Lighting", FormId::ModuleAmbientLighting)),
        is_connected.then_some(SettingsItem::form("Detection Sensor", FormId::ModuleDetectionSensor)),
        (is_connected && is_fw_version_gt_2_8).then_some(SettingsItem::form(
            "Traffic Management",
            FormId::ModuleTrafficManagement,
        )),
        (is_connected && is_fw_version_gt_2_8).then_some(SettingsItem::form("TAK", FormId::ModuleTak)),
        (is_connected && is_fw_version_gt_2_8).then_some(SettingsItem::form("Mesh Beacon", FormId::ModuleMeshBeacon)),
    ]
    .into_iter()
    .flatten()
    .collect()
}
