use std::collections::HashMap;
use std::sync::LazyLock;

use maplit::hashmap;
use meshtastic::protobufs::config::device_config::{RebroadcastMode, Role};
use meshtastic::protobufs::config::lo_ra_config::{ModemPreset, RegionCode};
use meshtastic::protobufs::config::{self, DeviceConfig, LoRaConfig};
use meshtastic::protobufs::{Config, ModuleConfig, User, from_radio};
use strum::IntoEnumIterator;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::name_of;
use crate::serde::{from_formdata, to_formdata};
use crate::types::{AppEvent, FormData, FormId, Toast};
use crate::types::{FormEnumVariant, FormItem, FormItemKind, SettingsItem};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
};

pub static SETTINGS: LazyLock<Vec<SettingsItem>> = LazyLock::new(|| build_settings());
pub static FORMS: LazyLock<HashMap<FormId, Vec<FormItem>>> = LazyLock::new(|| build_forms());

pub struct SettingsService {
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl SettingsService {
    pub fn new(
        app_event_rx: broadcast::Receiver<AppEvent>,
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
        meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
    ) -> Self {
        Self {
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
                Ok(event) = self.app_event_rx.recv() => self.handle_app_event(event).await?,
                Ok(event) = self.meshtastic_event_rx.recv() => self.handle_meshtastic_event(event)?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_app_event(&self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::SettingsFormSelected(id) => {
                self.state_action_tx
                    .send(StateAction::SettingsFormLoadingStart { id: id.clone() })?;

                match self.load_config(&id) {
                    Ok(data) => {
                        self.state_action_tx
                            .send(StateAction::SettingsFormLoadingDone {
                                id: id.clone(),
                                data,
                            })?
                    }
                    Err(e) => self
                        .state_action_tx
                        .send(StateAction::SettingsFormLoadingFail {
                            id: id.clone(),
                            error: e.to_string(),
                        })?,
                }
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
                self.state_action_tx
                    .send(StateAction::SettingsFormValueSet {
                        key: form_item.key,
                        value,
                    })?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_event(&self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            MeshtasticEvent::IncomingPacket(from_radio::PayloadVariant::Config(Config {
                payload_variant: Some(variant),
            })) => {
                self.state_action_tx
                    .send(StateAction::DeviceConfigSet(variant))?;
            }
            MeshtasticEvent::IncomingPacket(from_radio::PayloadVariant::ModuleConfig(
                ModuleConfig {
                    payload_variant: Some(variant),
                },
            )) => {
                self.state_action_tx
                    .send(StateAction::DeviceModuleConfigSet(variant))?;
            }
            MeshtasticEvent::ConfigSaveError(e) | MeshtasticEvent::UserSaveError(e) => {
                self.state_action_tx
                    .send(StateAction::Toast(Toast::error(e)))?;
            }
            MeshtasticEvent::ConfigSaved | MeshtasticEvent::UserSaved => {
                self.state_action_tx
                    .send(StateAction::Toast(Toast::success("config saved")))?;

                self.state_action_tx
                    .send(StateAction::SettingsFormSavingDone)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn load_config(&self, id: &FormId) -> anyhow::Result<FormData> {
        let state = &self.state_rx.borrow();

        let data = match id {
            FormId::RadioLora => to_formdata(
                state
                    .device_config
                    .lora
                    .as_ref()
                    .ok_or(anyhow::anyhow!("Lora config not loaded"))?,
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
                    .device_user
                    .as_ref()
                    .ok_or(anyhow::anyhow!("User config not loaded"))?,
            )?,
            _ => return Err(anyhow::anyhow!("Loader not implemented for FormId: {}", id)),
        };

        Ok(data)
    }

    fn save_config(&self, id: &FormId) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();
        let form_data = state.settings_form_data.as_ref().expect("should be Some");

        match id {
            FormId::RadioLora => {
                self.meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveConfig {
                        my_node_id: state.my_node_key.expect("should be Some"),
                        config: config::PayloadVariant::Lora(from_formdata::<LoRaConfig>(
                            &form_data,
                        )?),
                    })?;
            }
            FormId::DeviceDevice => {
                self.meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveConfig {
                        my_node_id: state.my_node_key.expect("should be Some"),
                        config: config::PayloadVariant::Device(from_formdata::<DeviceConfig>(
                            &form_data,
                        )?),
                    })?;
            }
            FormId::DeviceUser => {
                self.meshtastic_command_tx
                    .send(CommandToMeshtastic::SaveUser {
                        my_node_id: state.my_node_key.expect("should be Some"),
                        user: from_formdata::<User>(&form_data)?,
                    })?;
            }
            _ => unimplemented!(),
        };

        Ok(())
    }
}

fn build_forms<'a>() -> HashMap<FormId, Vec<FormItem>> {
    hashmap! {
        FormId::RadioLora => vec![
            FormItem::new(
                name_of!(region in LoRaConfig),
                "Region",
                Some("The region where you will be using your node."),
                FormItemKind::Enum(RegionCode::iter()
                    .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                    .collect()),
                |v| RegionCode::try_from(v.as_i32().expect("invalid FormValue"))
                        .and_then(|r| Ok(r.as_str_name().to_owned()))
                        .unwrap_or("?".to_owned()),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(use_preset in LoRaConfig),
                "Use Preset",
                Some("If enabled then \"Bandwidth\", \"Spread Factor\" and \"Coding Rate\" \
                      fields will be ignored."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(modem_preset in LoRaConfig),
                "Preset",
                Some("The field only makes sense if \"Use Preset\" field is set to true."),
                FormItemKind::Enum(ModemPreset::iter()
                    .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                    .collect()),
                |v| ModemPreset::try_from(v.as_i32().expect("invalid FormValue"))
                    .and_then(|r| Ok(r.as_str_name().to_owned()))
                    .unwrap_or("?".to_owned()),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(bandwidth in LoRaConfig),
                "Bandwidth *",
                Some("Certain bandwidth numbers are 'special' and will be converted to \
                      the appropriate floating point value: 31 -> 31.25 kHz. \
                      (*) The field only makes sense if \"Use Preset\" field is set to false."),
                FormItemKind::InputOfUnsignedInt32,
                |v| format!("{} kHz", v.to_string()),
                |v| (31..=500)
                    .contains(&v.as_u32().expect("invalid value"))
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Must be between 31 and 500"))

            ),
            FormItem::new(
                name_of!(spread_factor in LoRaConfig),
                "Spread Factor *",
                Some("A number from 5 to 12. Indicates number of chirps per symbol as \
                      1<<spread_factor. (*) The field only makes sense if \"Use Preset\" field \
                      is set to false."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| (5..=12)
                    .contains(&v.as_u32().expect("invalid value"))
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Must be between 5 and 12"))
            ),
            FormItem::new(
                name_of!(coding_rate in LoRaConfig),
                "Coding Rate *",
                Some("The denominator of the coding rate. (*) The field only makes sense \
                    if \"Use Preset\" field is set to false."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("4/5", 5 as u32),
                    FormEnumVariant::new("4/6", 6 as u32),
                    FormEnumVariant::new("4/7", 7 as u32),
                    FormEnumVariant::new("4/8", 8 as u32),
                ]),
                |v| format!("4/{}", v),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(ignore_mqtt in LoRaConfig),
                "Ignore MQTT",
                Some("If true, the device will not process any packets received via LoRa \
                      that passed via MQTT anywhere on the path towards it."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(config_ok_to_mqtt in LoRaConfig),
                "OK to MQTT",
                Some("Allow your packets to be published into MQTT."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(tx_enabled in LoRaConfig),
                "Transmit Enabled",
                Some("Disabling TX is useful for hot-swapping antennas and other tests."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(override_duty_cycle in LoRaConfig),
                "Override Duty Cycle",
                Some("If true, duty cycle limits will be exceeded and thus you're possibly \
                      not following the local regulations if you're not a HAM. Has no effect \
                      if the duty cycle of the used region is 100%."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(hop_limit in LoRaConfig),
                "Hops Limit",
                Some("Sets the maximum number of hops, default is 3. Increasing hops also \
                     increases congestion and should be used carefully. 0 hop broadcast messages \
                     will not get ACKs."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("0 hops", 0 as u32),
                    FormEnumVariant::new("1 hop", 1 as u32),
                    FormEnumVariant::new("2 hops", 2 as u32),
                    FormEnumVariant::new("3 hops", 3 as u32),
                    FormEnumVariant::new("4 hops", 4 as u32),
                    FormEnumVariant::new("5 hops", 5 as u32),
                    FormEnumVariant::new("6 hops", 6 as u32),
                    FormEnumVariant::new("7 hops", 7 as u32),
                ]),
                |v| format!("{} hop(s)", v.to_string()),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(channel_num in LoRaConfig),
                "Frequency Slot",
                Some("Your node's operating frequency is calculated based on the region, \
                      modem preset, and this field. When 0, the slot is automatically calculated \
                      based on the primary channel name and will change from the default \
                      public slot. Change back to the public default slot if private primary \
                      and public secondary channels are configured."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| (0..=20)
                    .contains(&v.as_u32().expect("invalid value"))
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Must be between 0 and 20"))
            ),
            FormItem::new(
                name_of!(sx126x_rx_boosted_gain in LoRaConfig),
                "RX Boosted Gain",
                Some("This is an option specific to the SX126x chip series which allows \
                      the chip to consume a small amount of additional power to \
                      increase RX sensitivity."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(override_frequency in LoRaConfig),
                "Frequency Override",
                Some("This parameter is for advanced users and licensed HAM radio operators. \
                      When enabled, the channel calculation will be ignored, and the set \
                      frequency will be used instead (frequency_offset still applies). \
                      This will allow you to use out-of-band frequencies."),
                FormItemKind::InputOfFloat32,
                |v| if v.as_f32().expect("invalid value") > 0.0 { format!("{} MHz", v.to_string()) } else { "not set".to_owned() },
                |v| (0.0..=2500.0)
                    .contains(&v.as_f32().expect("invalid value"))
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Must be between 0 and 2500"))
            ),
            FormItem::new(
                name_of!(tx_power in LoRaConfig),
                "Transmit Power",
                Some("In dBm. If zero, then use default max legal continuous power (i.e. something \
                      that won't burn out the radio hardware)."),
                FormItemKind::InputOfInt32,
                |v| format!("{} dBm", v.to_string()),
                |v| (-100..=100)
                    .contains(&v.as_i32().expect("invalid value"))
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Must be between -100 and 100"))
            ),
        ],
        FormId::DeviceUser => vec![
            FormItem::new(
                name_of!(long_name in User),
                "Long Name",
                Some("Full name of your node."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| (1..=38)
                    .contains(&v.as_string().expect("invalid value").len())
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Min length is 1, max 38"))
            ),
            FormItem::new(
                name_of!(short_name in User),
                "Short Name",
                Some("Short name of your node."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| (1..=4)
                    .contains(&v.as_string().expect("invalid value").len())
                    .then_some(())
                    .ok_or(anyhow::anyhow!("Min length is 1, max 4"))
            ),
            FormItem::new(
                name_of!(is_unmessagable in User),
                "Unmessagable",
                Some("Whether or not the node can be messaged."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(is_licensed in User),
                "Licensed (HAM)",
                Some("Enabling this option disables encryption and is not compatible with \
                      the default Meshtastic network."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            ),
        ],
        FormId::DeviceDevice => vec![
            FormItem::new(
                name_of!(role in DeviceConfig),
                "Device Role",
                None,
                FormItemKind::Enum(Role::iter()
                    .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                    .collect()),
                |v| Role::try_from(v.as_i32().expect("invalid FormValue"))
                        .and_then(|r| Ok(r.as_str_name().to_owned()))
                        .unwrap_or("?".to_owned()),
                |_| Ok(())
            ),
            FormItem::new(
                name_of!(rebroadcast_mode in DeviceConfig),
                "Rebroadcast Mode",
                None,
                FormItemKind::Enum(RebroadcastMode::iter()
                    .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                    .collect()),
                |v| RebroadcastMode::try_from(v.as_i32().expect("invalid FormValue"))
                        .and_then(|r| Ok(r.as_str_name().to_owned()))
                        .unwrap_or("?".to_owned()),
                |_| Ok(())
            ),
        ],
        FormId::AppUi => vec![
            FormItem::new(
                "paddings",
                "Hide global padding",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(())
            )
        ]
    }
}

fn build_settings() -> Vec<SettingsItem> {
    vec![
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
        // App
        SettingsItem::group("App"),
        SettingsItem::form("UI", FormId::AppUi),
    ]
}
