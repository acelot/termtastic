use std::collections::HashMap;
use std::sync::LazyLock;

use maplit::hashmap;
use meshtastic::protobufs::config::lo_ra_config::{ModemPreset, RegionCode};
use strum::IntoEnumIterator;
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;

use crate::types::{AppEvent, FormData, FormId, FormValue};
use crate::types::{FormEnumVariant, FormItem, FormItemKind, SettingsItem};
use crate::{
    meshtastic::types::{CommandToMeshtastic, MeshtasticEvent},
    state::{State, StateAction},
};

pub static SETTINGS: LazyLock<Vec<SettingsItem>> = LazyLock::new(|| build_settings());
pub static FORMS: LazyLock<HashMap<FormId, Vec<FormItem>>> = LazyLock::new(|| build_forms());

pub struct SettingsService {
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    meshtastic_command_tx: mpsc::UnboundedSender<CommandToMeshtastic>,
    meshtastic_event_rx: broadcast::Receiver<MeshtasticEvent>,
}

impl SettingsService {
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

                match self.make_form_data(&id) {
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
            AppEvent::SettingsFormLoadingCancelRequested => {
                self.state_action_tx.send(StateAction::SettingsFormClose)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_meshtastic_event(&self, event: MeshtasticEvent) -> anyhow::Result<()> {
        match event {
            MeshtasticEvent::IncomingConfig(variant) => {
                self.state_action_tx
                    .send(StateAction::DeviceConfigSet(variant))?;
            }
            MeshtasticEvent::IncomingModuleConfig(variant) => {
                self.state_action_tx
                    .send(StateAction::DeviceModuleConfigSet(variant))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn make_form_data(&self, id: &FormId) -> anyhow::Result<FormData> {
        let state = &self.state_rx.borrow();

        let data = match id {
            FormId::RadioLora => {
                let Some(lora) = &state.device_config.lora else {
                    return Err(anyhow::anyhow!("lora config not loaded"));
                };

                hashmap! {
                    "region" => FormValue::Int32(lora.region),
                    "preset" => FormValue::Int32(lora.modem_preset)
                }
            }
            _ => return Err(anyhow::anyhow!("unhandled FormId: {}", id)),
        };

        Ok(data)
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

fn build_forms() -> HashMap<FormId, Vec<FormItem>> {
    let lora_regions = RegionCode::iter()
        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
        .collect();

    let lora_presets = ModemPreset::iter()
        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
        .collect();

    hashmap! {
        FormId::RadioLora => vec![
            FormItem::new(
                "region",
                "Region",
                "Region where your node will work",
                FormItemKind::EnumOfInt32(lora_regions),
                |v| RegionCode::try_from(Into::<i32>::into(v))
                    .and_then(|r| Ok(r.as_str_name().to_owned()))
                    .unwrap_or("?".to_owned())
            ),
            FormItem::new(
                "preset",
                "Preset",
                "Radio preset",
                FormItemKind::EnumOfInt32(lora_presets),
                |v| ModemPreset::try_from(Into::<i32>::into(v))
                    .and_then(|r| Ok(r.as_str_name().to_owned()))
                    .unwrap_or("?".to_owned())
            ),
        ],
        FormId::AppUi => vec![
            FormItem::new("paddings", "Hide global padding", "", FormItemKind::Switch, bool_formatter)
        ]
    }
}

fn bool_formatter(v: &FormValue) -> String {
    if Into::<bool>::into(v) == true {
        "true".to_owned()
    } else {
        "false".to_owned()
    }
}
