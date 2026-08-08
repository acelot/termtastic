use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::LazyLock;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use hostaddr::HostAddr;
use meshtastic::protobufs::User;
use meshtastic::protobufs::config::bluetooth_config::PairingMode;
use meshtastic::protobufs::config::device_config::{RebroadcastMode, Role};
use meshtastic::protobufs::config::display_config::{CompassOrientation, DisplayMode, DisplayUnits, OledType};
use meshtastic::protobufs::config::lo_ra_config::{ModemPreset, RegionCode};
use meshtastic::protobufs::config::network_config::{AddressMode, IpV4Config, ProtocolFlags};
use meshtastic::protobufs::config::position_config::{GpsMode, PositionFlags};
use meshtastic::protobufs::config::{
    BluetoothConfig, DeviceConfig, DisplayConfig, LoRaConfig, NetworkConfig, PositionConfig, PowerConfig,
    SecurityConfig,
};
use meshtastic::protobufs::module_config::canned_message_config::InputEventChar;
use meshtastic::protobufs::module_config::detection_sensor_config::TriggerType;
use meshtastic::protobufs::module_config::serial_config::{SerialBaud, SerialMode};
use meshtastic::protobufs::module_config::{
    AmbientLightingConfig, CannedMessageConfig, DetectionSensorConfig, ExternalNotificationConfig, MapReportSettings,
    MqttConfig, NeighborInfoConfig, RangeTestConfig, SerialConfig, StoreForwardConfig, TelemetryConfig,
    TrafficManagementConfig,
};
use strum::IntoEnumIterator;

use crate::serde::to_formdata;
use crate::types::{
    AppEvent, Channel, FormBitMaskVariant, FormData, FormEnumVariant, FormId, FormItem, FormItemKey, FormItemKind,
    FormValue, UiConfig,
};
use nameof::name_of;

pub static FORMS: LazyLock<HashMap<FormId, Vec<FormItem>>> = LazyLock::new(build_forms);

static DEFAULT_MAP_REPORT_SETTINGS: LazyLock<FormData> =
    LazyLock::new(|| to_formdata(&MapReportSettings::default()).unwrap());

static DEFAULT_IPV4_CONFIG: LazyLock<FormData> = LazyLock::new(|| to_formdata(&IpV4Config::default()).unwrap());

macro_rules! channel_form {
    ($($x:expr),* $(,)?) => {
        {
            let mut vec: Vec<FormItem> = Vec::new();

            $(
                vec.push(
                    FormItem::new(
                        FormItemKey::Custom {
                            getter: |data| {
                                data.get($x)
                                    .expect("should exists")
                                    .as_nested()
                                    .get(name_of!(is_enabled in Channel))
                                    .cloned()
                                    .expect("should exists")
                            },
                            setter: |data, value| {
                                data.get_mut($x)
                                    .expect("should exists")
                                    .as_nested_mut()
                                    .insert(name_of!(is_enabled in Channel).to_owned(), value);
                            },
                        },
                        concat!("ENABLE CHANNEL #", $x),
                        Some("All channel fields will be erased if the channel is disabled."),
                        FormItemKind::Switch,
                        |v| v.to_string(),
                        |_| Ok(()),
                    )
                );

                vec.push(
                    FormItem::new(
                        FormItemKey::Custom {
                            getter: |data| {
                                data.get($x)
                                    .expect("should exists")
                                    .as_nested()
                                    .get(name_of!(name in Channel))
                                    .cloned()
                                    .expect("should exists")
                            },
                            setter: |data, value| {
                                data.get_mut($x)
                                    .expect("should exists")
                                    .as_nested_mut()
                                    .insert(name_of!(name in Channel).to_owned(), value);
                            },
                        },
                        concat!("  #", $x, " Name"),
                        Some("Short channel name. Maximum 10 characters."),
                        FormItemKind::InputOfString,
                        |v| v.to_string(),
                        |v| (0..=10)
                            .contains(&v.as_string().len())
                            .then_some(())
                            .ok_or(anyhow::anyhow!("Max length is 10")),
                    )
                );

                vec.push(
                    FormItem::new(
                        FormItemKey::Custom {
                            getter: |data| {
                                data.get($x)
                                    .expect("should exists")
                                    .as_nested()
                                    .get(name_of!(psk in Channel))
                                    .cloned()
                                    .expect("should exists")
                            },
                            setter: |data, value| {
                                data.get_mut($x)
                                    .expect("should exists")
                                    .as_nested_mut()
                                    .insert(name_of!(psk in Channel).to_owned(), value);
                            },
                        },
                        concat!("  #", $x, " PSK"),
                        Some("Base64 encoded string. Leave empty to disable encryption."),
                        FormItemKind::InputOfBase64,
                        |v| BASE64_STANDARD.encode(v.as_vec().iter().map(|v| v.as_u8()).collect::<Vec<u8>>()),
                        |v| (0..=32)
                            .contains(&v.as_vec().len())
                            .then_some(())
                            .ok_or(anyhow::anyhow!("Max length is 32")),
                    )
                );

                vec.push(
                    FormItem::new(
                        FormItemKey::Custom {
                            getter: |_| FormValue::Option(None),
                            setter: |data, value| {
                                data.get_mut($x)
                                    .expect("should exists")
                                    .as_nested_mut()
                                    .insert(name_of!(psk in Channel).to_owned(), value);
                            },
                        },
                        concat!("  #", $x, " Generate PSK"),
                        Some("Will generate random 16-byte PSK."),
                        FormItemKind::Button(|_| {
                            let key: Vec<u8> = std::iter::repeat_with(|| fastrand::u8(..)).take(16).collect();
                            FormValue::from(key)
                        }),
                        |_| "<GENERATE>".to_owned(),
                        |_| Ok(()),
                    ),
                );

                vec.push(
                    FormItem::new(
                        FormItemKey::Custom {
                            getter: |data| {
                                data.get($x)
                                    .expect("should exists")
                                    .as_nested()
                                    .get(name_of!(uplink_enabled in Channel))
                                    .cloned()
                                    .expect("should exists")
                            },
                            setter: |data, value| {
                                data.get_mut($x)
                                    .expect("should exists")
                                    .as_nested_mut()
                                    .insert(name_of!(uplink_enabled in Channel).to_owned(), value);
                            },
                        },
                        concat!("  #", $x, " Uplink Enabled"),
                        Some(
                            "If enabled, messages from the mesh will be sent to the public \
                            internet through any node's configured gateway."
                        ),
                        FormItemKind::Switch,
                        |v| v.to_string(),
                        |_| Ok(()),
                    )
                );

                vec.push(
                    FormItem::new(
                        FormItemKey::Custom {
                            getter: |data| {
                                data.get($x)
                                    .expect("should exists")
                                    .as_nested()
                                    .get(name_of!(downlink_enabled in Channel))
                                    .cloned()
                                    .expect("should exists")
                            },
                            setter: |data, value| {
                                data.get_mut($x)
                                    .expect("should exists")
                                    .as_nested_mut()
                                    .insert(name_of!(downlink_enabled in Channel).to_owned(), value);
                            },
                        },
                        concat!("  #", $x, " Downlink Enabled"),
                        Some(
                            "If enabled, messages captured from a public internet gateway \
                            will be forwarded to the local mesh."
                        ),
                        FormItemKind::Switch,
                        |v| v.to_string(),
                        |_| Ok(()),
                    )
                );
            )*

            vec
        }
    };
}

fn build_forms() -> HashMap<FormId, Vec<FormItem>> {
    let mut forms = HashMap::new();

    forms.insert(
        FormId::AppUi,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(is_top_padding_hidden in UiConfig)),
                "Window: Hide top padding",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(is_bottom_padding_hidden in UiConfig)),
                "Window: Hide bottom padding",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(is_left_padding_hidden in UiConfig)),
                "Window: Hide left padding",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(is_right_padding_hidden in UiConfig)),
                "Window: Hide right padding",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::RadioLora,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(region in LoRaConfig)),
                "Region",
                Some("The region where you will be using your node."),
                FormItemKind::Enum(
                    RegionCode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    RegionCode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(use_preset in LoRaConfig)),
                "Use Preset",
                Some("If enabled then \"Bandwidth\", \"Spread Factor\" and \"Coding Rate\" fields will be ignored."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(modem_preset in LoRaConfig)),
                "Preset",
                Some("The field only makes sense if \"Use Preset\" field is set to true."),
                FormItemKind::Enum(
                    ModemPreset::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    ModemPreset::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(bandwidth in LoRaConfig)),
                "Bandwidth *",
                Some(
                    "Certain bandwidth numbers are 'special' and will be converted to the appropriate floating point \
                    value: 31 -> 31.25 kHz. (*) The field only makes sense if \"Use Preset\" field is set to false.",
                ),
                FormItemKind::InputOfUnsignedInt32,
                |v| format!("{} kHz", v),
                |v| {
                    (31..=500)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 31 and 500"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(spread_factor in LoRaConfig)),
                "Spread Factor *",
                Some(
                    "A number from 5 to 12. Indicates number of chirps per symbol as 1<<spread_factor. (*) The field \
                    only makes sense if \"Use Preset\" field is set to false.",
                ),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (5..=12)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 5 and 12"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(coding_rate in LoRaConfig)),
                "Coding Rate *",
                Some(
                    "The denominator of the coding rate. (*) The field only makes sense if \"Use Preset\" field is \
                    set to false.",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("4/5", 5u32),
                    FormEnumVariant::new("4/6", 6u32),
                    FormEnumVariant::new("4/7", 7u32),
                    FormEnumVariant::new("4/8", 8u32),
                ]),
                |v| format!("4/{}", v),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(ignore_mqtt in LoRaConfig)),
                "Ignore MQTT",
                Some(
                    "If true, the device will not process any packets received via LoRa \
                      that passed via MQTT anywhere on the path towards it.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(config_ok_to_mqtt in LoRaConfig)),
                "OK to MQTT",
                Some("Allow your packets to be published into MQTT."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(tx_enabled in LoRaConfig)),
                "Transmit Enabled",
                Some("Disabling TX is useful for hot-swapping antennas and other tests."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(override_duty_cycle in LoRaConfig)),
                "Override Duty Cycle",
                Some(
                    "If true, duty cycle limits will be exceeded and thus you're possibly not following the local \
                    regulations if you're not a HAM. Has no effect if the duty cycle of the used region is 100%.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(hop_limit in LoRaConfig)),
                "Hops Limit",
                Some(
                    "Sets the maximum number of hops, default is 3. Increasing hops also increases congestion and \
                    should be used carefully. 0 hop broadcast messages will not get ACKs.",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("0 hops", 0u32),
                    FormEnumVariant::new("1 hop", 1u32),
                    FormEnumVariant::new("2 hops", 2u32),
                    FormEnumVariant::new("3 hops", 3u32),
                    FormEnumVariant::new("4 hops", 4u32),
                    FormEnumVariant::new("5 hops", 5u32),
                    FormEnumVariant::new("6 hops", 6u32),
                    FormEnumVariant::new("7 hops", 7u32),
                ]),
                |v| format!("{} hop(s)", v),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(channel_num in LoRaConfig)),
                "Frequency Slot",
                Some(
                    "Your node's operating frequency is calculated based on the region, modem preset, and this field. \
                    When 0, the slot is automatically calculated based on the primary channel name and will change \
                    from the default public slot. Change back to the public default slot if private primary and \
                    public secondary channels are configured.",
                ),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=20)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 20"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(sx126x_rx_boosted_gain in LoRaConfig)),
                "RX Boosted Gain",
                Some(
                    "This is an option specific to the SX126x chip series which allows the chip to consume a small \
                    amount of additional power to increase RX sensitivity.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(override_frequency in LoRaConfig)),
                "Frequency Override",
                Some(
                    "This parameter is for advanced users and licensed HAM radio operators. When enabled, the \
                    channel calculation will be ignored, and the set frequency will be used instead \
                    (frequency_offset still applies). This will allow you to use out-of-band frequencies.",
                ),
                FormItemKind::InputOfFloat32,
                |v| {
                    if v.as_f32() > 0.0 {
                        format!("{} MHz", v)
                    } else {
                        "not set".to_owned()
                    }
                },
                |v| {
                    (0.0..=2500.0)
                        .contains(&v.as_f32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 2500"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(tx_power in LoRaConfig)),
                "Transmit Power",
                Some(
                    "In dBm. If zero, then use default max legal continuous power (i.e. something that won't burn \
                    out the radio hardware).",
                ),
                FormItemKind::InputOfInt32,
                |v| format!("{} dBm", v),
                |v| {
                    (-100..=100)
                        .contains(&v.as_i32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between -100 and 100"))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::RadioChannels,
        channel_form!("0", "1", "2", "3", "4", "5", "6", "7"),
    );

    forms.insert(
        FormId::RadioSecurity,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(public_key in SecurityConfig)),
                "Public Key",
                Some(
                    "The public key of the user's device. Sent out to other nodes on the mesh to allow them \
                    to compute a shared secret key.",
                ),
                FormItemKind::InputOfBase64,
                |v| BASE64_STANDARD.encode(v.as_vec().iter().map(|v| v.as_u8()).collect::<Vec<u8>>()),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(private_key in SecurityConfig)),
                "Private Key",
                Some("The private key of the device. Used to create a shared key with a remote device."),
                FormItemKind::InputOfBase64,
                |v| BASE64_STANDARD.encode(v.as_vec().iter().map(|v| v.as_u8()).collect::<Vec<u8>>()),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(private_key in SecurityConfig))
                            .cloned()
                            .expect("should exists")
                    },
                    setter: |data, value| {
                        data.insert(name_of!(private_key in SecurityConfig).to_owned(), value);
                    },
                },
                "Generate private key",
                Some("WARNING! To communicate with other nodes, you need to exchange keys again."),
                FormItemKind::Button(|_| {
                    let key: Vec<u8> = std::iter::repeat_with(|| fastrand::u8(..)).take(32).collect();
                    FormValue::from(key)
                }),
                |_| "<REGENERATE>".to_owned(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(admin_key in SecurityConfig))
                            .expect("should exists")
                            .as_vec().first()
                            .cloned()
                            .unwrap_or(FormValue::Vec(vec![]))
                    },
                    setter: |data, value| {
                        let mut keys = data
                            .get(name_of!(admin_key in SecurityConfig))
                            .expect("should exists")
                            .as_vec()
                            .clone();

                        if keys.len() < 3 {
                            keys.resize(3, FormValue::Vec(vec![]));
                        }

                        keys[0] = value;
                        data.insert(name_of!(admin_key in SecurityConfig).to_owned(), FormValue::Vec(keys));
                    },
                },
                "Admin Key #1",
                Some("The public key authorized to send admin messages to this node."),
                FormItemKind::InputOfBase64,
                |v| BASE64_STANDARD.encode(v.as_vec().iter().map(|v| v.as_u8()).collect::<Vec<u8>>()),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(admin_key in SecurityConfig))
                            .expect("should exists")
                            .as_vec()
                            .get(1)
                            .cloned()
                            .unwrap_or(FormValue::Vec(vec![]))
                    },
                    setter: |data, value| {
                        let mut keys = data
                            .get(name_of!(admin_key in SecurityConfig))
                            .expect("should exists")
                            .as_vec()
                            .clone();

                        if keys.len() < 3 {
                            keys.resize(3, FormValue::Vec(vec![]));
                        }

                        keys[1] = value;
                        data.insert(name_of!(admin_key in SecurityConfig).to_owned(), FormValue::Vec(keys));
                    },
                },
                "Admin Key #2",
                Some("The public key authorized to send admin messages to this node."),
                FormItemKind::InputOfBase64,
                |v| BASE64_STANDARD.encode(v.as_vec().iter().map(|v| v.as_u8()).collect::<Vec<u8>>()),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(admin_key in SecurityConfig))
                            .expect("should exists")
                            .as_vec()
                            .get(2)
                            .cloned()
                            .unwrap_or(FormValue::Vec(vec![]))
                    },
                    setter: |data, value| {
                        let mut keys = data
                            .get(name_of!(admin_key in SecurityConfig))
                            .expect("should exists")
                            .as_vec()
                            .clone();

                        if keys.len() < 3 {
                            keys.resize(3, FormValue::Vec(vec![]));
                        }

                        keys[2] = value;
                        data.insert(name_of!(admin_key in SecurityConfig).to_owned(), FormValue::Vec(keys));
                    },
                },
                "Admin Key #3",
                Some("The public key authorized to send admin messages to this node."),
                FormItemKind::InputOfBase64,
                |v| BASE64_STANDARD.encode(v.as_vec().iter().map(|v| v.as_u8()).collect::<Vec<u8>>()),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(serial_enabled in SecurityConfig)),
                "Serial Console",
                Some("Serial Console over the Stream API."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(debug_log_api_enabled in SecurityConfig)),
                "Debug Log API Enabled",
                Some(
                    "By default we turn off logging as soon as an API client connects (to keep shared serial \
                    link quiet). Output live debug logging over serial or bluetooth is set to true.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(admin_channel_enabled in SecurityConfig)),
                "Legacy Admin Channel",
                Some("Allow incoming device control over the insecure legacy admin channel."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::DeviceUser,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(long_name in User)),
                "Long Name",
                Some("Full name of your node."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    (1..=38)
                        .contains(&v.as_string().len())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Min length is 1, max 38"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(short_name in User)),
                "Short Name",
                Some("Short name of your node."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    (1..=4)
                        .contains(&v.as_string().len())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Min length is 1, max 4"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(is_unmessagable in User)),
                "Unmessagable",
                Some("Whether or not the node can be messaged."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(is_licensed in User)),
                "Licensed (HAM)",
                Some(
                    "Enabling this option disables encryption and is not compatible with the default Meshtastic \
                    network.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::None,
                "Broadcast my NodeInfo",
                None,
                FormItemKind::Action(AppEvent::NodeInfoBroadcastRequested),
                |_| "<Send>".to_owned(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::DeviceDevice,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(role in DeviceConfig)),
                "Device Role",
                None,
                FormItemKind::Enum(
                    Role::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    Role::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rebroadcast_mode in DeviceConfig)),
                "Rebroadcast Mode",
                None,
                FormItemKind::Enum(
                    RebroadcastMode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    RebroadcastMode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(node_info_broadcast_secs in DeviceConfig)),
                "NodeInfo Broadcast Interval",
                None,
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    if secs > 0 {
                        format!("{} hours", secs / 3600)
                    } else {
                        "Unset".to_owned()
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(double_tap_as_button_press in DeviceConfig)),
                "Double Tap as Button",
                Some("Treat double tap interrupt on supported accelerometers as a button press if set to true."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(disable_triple_click in DeviceConfig)),
                "Triple Click Ad Hoc Ping",
                Some("Disables the triple-press of user button to enable or disable GPS."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(led_heartbeat_disabled in DeviceConfig)),
                "Disable LED Heartbeat",
                Some("If true, disable the default blinking LED (LED_PIN) behavior on the device."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(tzdef in DeviceConfig)),
                "Time Zone",
                Some("POSIX Timezone definition string."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(button_gpio in DeviceConfig)),
                "Button GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(buzzer_gpio in DeviceConfig)),
                "Buzzer GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::DevicePosition,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(position_broadcast_secs in PositionConfig)),
                "Broadcast Interval",
                Some(
                    "The maximum interval that can elapse without a node broadcasting a position. Default 15 minutes.",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Default", 0u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("90 seconds", 90u32),
                    FormEnumVariant::new("5 minutes", 300u32),
                    FormEnumVariant::new("15 minutes", 900u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Default".to_string(),
                        60 => "1 minute".to_string(),
                        90 => "90 seconds".to_string(),
                        1..3600 => format!("{} minutes", secs / 60),
                        3600 => "1 hour".to_string(),
                        3601..=u32::MAX => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(position_broadcast_smart_enabled in PositionConfig)),
                "Smart Position (SP)",
                Some("Adaptive position broadcast."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(broadcast_smart_minimum_interval_secs in PositionConfig)),
                "SP Minimum Interval",
                Some("The minimum number of seconds (since the last send) before we can send a position."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Default", 0u32),
                    FormEnumVariant::new("15 seconds", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("45 seconds", 45u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("5 minutes", 300u32),
                    FormEnumVariant::new("10 minutes", 600u32),
                    FormEnumVariant::new("15 minutes", 900u32),
                    FormEnumVariant::new("30 minutes", 1800u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Default".to_string(),
                        1..60 => format!("{} seconds", secs),
                        60 => "1 minute".to_string(),
                        61..3600 => format!("{} minutes", secs / 60),
                        3600 => "1 hour".to_string(),
                        3600..=u32::MAX => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(broadcast_smart_minimum_distance in PositionConfig)),
                "SP Minimum Distance",
                Some("The minimum distance in meters traveled (since the last send) before we can send a position."),
                FormItemKind::InputOfUnsignedInt32,
                |v| format!("{} meters", v.as_u32()),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(fixed_position in PositionConfig)),
                "Fixed Position",
                Some("If set, this node is at a fixed position."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(gps_mode in PositionConfig)),
                "GPS Mode",
                Some("Set where GPS is enabled, disabled, or not present."),
                FormItemKind::Enum(
                    GpsMode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    GpsMode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(gps_update_interval in PositionConfig)),
                "GPS Update Interval",
                Some("How often should we try to get GPS position (in seconds). Default once every 30 seconds."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Default", 0u32),
                    FormEnumVariant::new("8 seconds", 8u32),
                    FormEnumVariant::new("20 seconds", 20u32),
                    FormEnumVariant::new("40 seconds", 40u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("80 seconds", 80u32),
                    FormEnumVariant::new("2 minutes", 120u32),
                    FormEnumVariant::new("5 minutes", 300u32),
                    FormEnumVariant::new("10 minutes", 600u32),
                    FormEnumVariant::new("15 minutes", 900u32),
                    FormEnumVariant::new("30 minutes", 1800u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Default".to_string(),
                        60 => "1 minute".to_string(),
                        1..120 => format!("{} seconds", secs),
                        120..3600 => format!("{} minutes", secs / 60),
                        3600 => "1 hour".to_string(),
                        3600..=u32::MAX => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(position_flags in PositionConfig)),
                "Position Flags",
                Some("Bit field of boolean configuration options for POSITION messages."),
                FormItemKind::BitMask(
                    PositionFlags::iter()
                        .filter(|v| v != &PositionFlags::Unset)
                        .map(|v| FormBitMaskVariant::new(v.as_str_name(), v as u32))
                        .collect(),
                ),
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rx_gpio in PositionConfig)),
                "GPS RX GPIO",
                Some("GPS_RX_PIN for your board."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(tx_gpio in PositionConfig)),
                "GPS TX GPIO",
                Some("GPS_TX_PIN for your board."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(gps_en_gpio in PositionConfig)),
                "GPS EN GPIO",
                Some("PIN_GPS_EN for your board."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::DevicePower,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(is_power_saving in PowerConfig)),
                "Power Saving Mode",
                Some(
                    "Will sleep everything as mush as possible, for the tracker ad sensor role this will also \
                    include the Lora radio. Don't use this setting if you want to use your device with the phone \
                    apps or are using a device without a power button.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(on_battery_shutdown_after_secs in PowerConfig)),
                "Shutdown on Power Loss",
                Some("If non-zero, the device will fully power off this many seconds after external power is removed."),
                FormItemKind::InputOfUnsignedInt32,
                |v| match v.as_u32() {
                    0 => "Always On".to_owned(),
                    _ => format!("{} secs", v),
                },
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(adc_multiplier_override in PowerConfig)),
                "ADC Multiplier Override",
                Some(
                    "Ratio of voltage divider for battery pin eg. 3.20 (R1=100k, R2=220k). Overrides the \
                    ADC_MULTIPLIER defined in variant for battery voltage calculation. 0 – disable override.",
                ),
                FormItemKind::InputOfFloat32,
                |v| match v.as_f32() {
                    0.0 => "Disabled".to_owned(),
                    _ => v.to_string(),
                },
                |v| {
                    let v = v.as_f32();
                    if v == 0.0 {
                        return Ok(());
                    }

                    (2.0..=6.0)
                        .contains(&v)
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 2.0 and 6.0, or 0 for disable"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(wait_bluetooth_secs in PowerConfig)),
                "Wait for Bluetooth Timeout",
                Some("The number of seconds for to wait before turning off BLE in No Bluetooth states."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("1 second", 1u32),
                    FormEnumVariant::new("5 seconds", 5u32),
                    FormEnumVariant::new("10 seconds", 10u32),
                    FormEnumVariant::new("15 seconds", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_string(),
                        1 => "1 second".to_string(),
                        60 => "1 minute".to_string(),
                        1..=u32::MAX => format!("{} seconds", secs),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(sds_secs in PowerConfig)),
                "Super Deep Sleep Duration",
                Some(
                    "While in Light Sleep if mesh_sds_timeout_secs is exceeded we will lower into super deep sleep \
                    for this value (default 1 year) or a button press. 0 for default of one year.",
                ),
                FormItemKind::InputOfUnsignedInt32,
                |v| match v.as_u32() {
                    0 => "Default".to_owned(),
                    _ => format!("{} secs", v),
                },
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(min_wake_secs in PowerConfig)),
                "Minimum Wake Time",
                Some(
                    "While in light sleep when we receive packets on the LoRa radio we will wake and handle them \
                    and stay awake in no BLE mode for this value.",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("1 second", 1u32),
                    FormEnumVariant::new("5 seconds", 5u32),
                    FormEnumVariant::new("10 seconds", 10u32),
                    FormEnumVariant::new("15 seconds", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_string(),
                        1 => "1 second".to_string(),
                        60 => "1 minute".to_string(),
                        1..=u32::MAX => format!("{} seconds", secs),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(device_battery_ina_address in PowerConfig)),
                "Battery INA_2XX I2C Address",
                Some("I2C address of INA_2XX to use for reading device battery voltage."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::DeviceDisplay,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(use_12h_clock in DisplayConfig)),
                "Use 12h Clock Format",
                Some("When enabled, the device will display the time in 12-hour format on screen."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(heading_bold in DisplayConfig)),
                "Bold Heading",
                Some("Bold the heading text on the screen."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(units in DisplayConfig)),
                "Display Units",
                None,
                FormItemKind::Enum(
                    DisplayUnits::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    DisplayUnits::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(screen_on_secs in DisplayConfig)),
                "Screen On For",
                Some("Number of seconds the screen stays on after pressing the user button or receiving a message."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Always On", 0u32),
                    FormEnumVariant::new("15 second", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("5 minutes", 5 * 60u32),
                    FormEnumVariant::new("10 minutes", 10 * 60u32),
                    FormEnumVariant::new("15 minutes", 15 * 60u32),
                    FormEnumVariant::new("30 minutes", 30 * 60u32),
                    FormEnumVariant::new("1 hour", 60 * 60u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Always On".to_string(),
                        1..60 => format!("{} seconds", secs),
                        60 => "1 minute".to_string(),
                        3600 => "1 hour".to_string(),
                        61..=u32::MAX => format!("{} minutes", secs / 60),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(auto_screen_carousel_secs in DisplayConfig)),
                "Carousel Interval",
                Some(
                    "Automatically toggles to the next page on the screen like a carousel, based the specified \
                     interval in seconds.",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("15 second", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("5 minutes", 5 * 60u32),
                    FormEnumVariant::new("10 minutes", 10 * 60u32),
                    FormEnumVariant::new("15 minutes", 15 * 60u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_string(),
                        1..60 => format!("{} seconds", secs),
                        60 => "1 minute".to_string(),
                        61..=u32::MAX => format!("{} minutes", secs / 60),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(wake_on_tap_or_motion in DisplayConfig)),
                "Wake On Tap or Motion",
                Some("Requires that there be an accelerometer on your device."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(flip_screen in DisplayConfig)),
                "Flip Screen",
                Some("Flip screen vertically."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(displaymode in DisplayConfig)),
                "Display Mode",
                None,
                FormItemKind::Enum(
                    DisplayMode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    DisplayMode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(oled in DisplayConfig)),
                "OLED Type",
                None,
                FormItemKind::Enum(
                    OledType::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    OledType::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(compass_orientation in DisplayConfig)),
                "Compass Orientation",
                None,
                FormItemKind::Enum(
                    CompassOrientation::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    CompassOrientation::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::DeviceBluetooth,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in BluetoothConfig)),
                "Bluetooth Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(mode in BluetoothConfig)),
                "Pairing Mode",
                None,
                FormItemKind::Enum(
                    PairingMode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    PairingMode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(fixed_pin in BluetoothConfig)),
                "Fixed PIN",
                Some("6-digit PIN code."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (100_000..=999_999)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Invalid PIN"))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::DeviceNetwork,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(wifi_enabled in NetworkConfig)),
                "Wi-Fi Enabled",
                Some("Enable WiFi (disables Bluetooth)."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(wifi_ssid in NetworkConfig)),
                "SSID",
                Some("If set, this node will try to join the specified wifi network and acquire an address via DHCP."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    (0..=32)
                        .contains(&v.as_string().len())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Max length is 32"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(wifi_psk in NetworkConfig)),
                "Password",
                Some("SSID Preshared Key."),
                FormItemKind::InputOfString,
                |v| "*".repeat(v.as_string().len()),
                |v| {
                    (0..=64)
                        .contains(&v.as_string().len())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Max length is 64"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(ntp_server in NetworkConfig)),
                "NTP Server",
                Some("NTP server to use if Wi-Fi is connected, defaults to meshtastic.pool.ntp.org."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    v.as_string()
                        .parse::<HostAddr<String>>()
                        .map(|_| ())
                        .map_err(Into::into)
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rsyslog_server in NetworkConfig)),
                "Rsyslog Server",
                Some("rsyslog server and port."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    v.as_string()
                        .parse::<HostAddr<String>>()
                        .map(|_| ())
                        .map_err(Into::into)
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(address_mode in NetworkConfig)),
                "IPv4 Mode",
                None,
                FormItemKind::Enum(
                    AddressMode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    AddressMode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(ipv4_config in NetworkConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| v.as_nested().get(name_of!(ip in IpV4Config))).map(|v| FormValue::String(Ipv4Addr::from(v.as_u32()).to_string()))
                            .unwrap_or(FormValue::String("0.0.0.0".to_owned()))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(ipv4_config in NetworkConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(DEFAULT_IPV4_CONFIG.clone())))),
                            );
                        }

                        let ipv4 = value.as_string().parse::<Ipv4Addr>().expect("should parse");

                        data.get_mut(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(
                                name_of!(ip in IpV4Config).to_owned(),
                                FormValue::UnsignedInt32(ipv4.to_bits()),
                            );
                    },
                },
                "IP *",
                Some("(*) The field only makes sense if \"IPv4 Mode\" field is set to \"STATIC\""),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| v.as_string().parse::<Ipv4Addr>().map(|_| ()).map_err(Into::into),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(ipv4_config in NetworkConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| v.as_nested().get(name_of!(gateway in IpV4Config))).map(|v| FormValue::String(Ipv4Addr::from(v.as_u32()).to_string()))
                            .unwrap_or(FormValue::String("0.0.0.0".to_owned()))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(ipv4_config in NetworkConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(DEFAULT_IPV4_CONFIG.clone())))),
                            );
                        }

                        let ipv4 = value.as_string().parse::<Ipv4Addr>().expect("should parse");

                        data.get_mut(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(
                                name_of!(gateway in IpV4Config).to_owned(),
                                FormValue::UnsignedInt32(ipv4.to_bits()),
                            );
                    },
                },
                "Gateway *",
                Some("(*) The field only makes sense if \"IPv4 Mode\" field is set to \"STATIC\""),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| v.as_string().parse::<Ipv4Addr>().map(|_| ()).map_err(Into::into),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(ipv4_config in NetworkConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| v.as_nested().get(name_of!(subnet in IpV4Config))).map(|v| FormValue::String(Ipv4Addr::from(v.as_u32()).to_string()))
                            .unwrap_or(FormValue::String("0.0.0.0".to_owned()))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(ipv4_config in NetworkConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(DEFAULT_IPV4_CONFIG.clone())))),
                            );
                        }

                        let ipv4 = value.as_string().parse::<Ipv4Addr>().expect("should parse");

                        data.get_mut(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(
                                name_of!(subnet in IpV4Config).to_owned(),
                                FormValue::UnsignedInt32(ipv4.to_bits()),
                            );
                    },
                },
                "Subnet *",
                Some("(*) The field only makes sense if \"IPv4 Mode\" field is set to \"STATIC\""),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| v.as_string().parse::<Ipv4Addr>().map(|_| ()).map_err(Into::into),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(ipv4_config in NetworkConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| v.as_nested().get(name_of!(dns in IpV4Config))).map(|v| FormValue::String(Ipv4Addr::from(v.as_u32()).to_string()))
                            .unwrap_or(FormValue::String("0.0.0.0".to_owned()))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(ipv4_config in NetworkConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(DEFAULT_IPV4_CONFIG.clone())))),
                            );
                        }

                        let ipv4 = value.as_string().parse::<Ipv4Addr>().expect("should parse");

                        data.get_mut(name_of!(ipv4_config in NetworkConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(
                                name_of!(dns in IpV4Config).to_owned(),
                                FormValue::UnsignedInt32(ipv4.to_bits()),
                            );
                    },
                },
                "DNS *",
                Some("(*) The field only makes sense if \"IPv4 Mode\" field is set to \"STATIC\""),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| v.as_string().parse::<Ipv4Addr>().map(|_| ()).map_err(Into::into),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(eth_enabled in NetworkConfig)),
                "Ethernet Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(ipv6_enabled in NetworkConfig)),
                "IPv6 Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled_protocols in NetworkConfig)),
                "Enabled Protocols",
                Some("Defines which auxiliary network protocols are used to send packets."),
                FormItemKind::BitMask(
                    ProtocolFlags::iter()
                        .filter(|v| v != &ProtocolFlags::NoBroadcast)
                        .map(|v| FormBitMaskVariant::new(v.as_str_name(), v as u32))
                        .collect(),
                ),
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::DeviceAdministration,
        Vec::from([
            FormItem::new(
                FormItemKey::None,
                "Reboot Device",
                None,
                FormItemKind::Action(AppEvent::DeviceRebootRequested),
                |_| "<REBOOT>".to_owned(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::None,
                "Shutdown Device",
                None,
                FormItemKind::Action(AppEvent::DeviceShutdownRequested),
                |_| "<SHUTDOWN>".to_owned(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleMqtt,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in MqttConfig)),
                "MQTT Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(address in MqttConfig)),
                "Address",
                Some("The server to use for our MQTT global message gateway feature."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    v.as_string()
                        .parse::<HostAddr<String>>()
                        .map(|_| ())
                        .map_err(Into::into)
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(username in MqttConfig)),
                "Username",
                None,
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(password in MqttConfig)),
                "Password",
                None,
                FormItemKind::InputOfString,
                |v| "*".repeat(v.as_string().len()),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(encryption_enabled in MqttConfig)),
                "Encryption Enabled",
                Some("Whether to send encrypted or decrypted packets to MQTT."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(json_enabled in MqttConfig)),
                "JSON Output Enabled",
                Some("Whether to send / consume json packets on MQTT."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(tls_enabled in MqttConfig)),
                "TLS Enabled",
                Some("If true, we attempt to establish a secure connection using TLS."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(root in MqttConfig)),
                "Root Topic",
                Some("The root topic to use for MQTT messages. Default is \"msh\"."),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(proxy_to_client_enabled in MqttConfig)),
                "Proxy to Client Enabled",
                Some(
                    "If true, we can use the connected phone / client to proxy messages to MQTT instead of \
                     a direct connection.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(map_reporting_enabled in MqttConfig)),
                "Map Reporting Enabled",
                Some("If true, we will periodically report unencrypted information about our node to a map via MQTT."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(map_report_settings in MqttConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| {
                                v.as_nested()
                                    .get(name_of!(should_report_location in MapReportSettings))
                                    .cloned()
                            })
                            .unwrap_or(FormValue::Bool(false))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(map_report_settings in MqttConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(map_report_settings in MqttConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(
                                    DEFAULT_MAP_REPORT_SETTINGS.clone(),
                                )))),
                            );
                        }

                        data.get_mut(name_of!(map_report_settings in MqttConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(name_of!(should_report_location in MapReportSettings).to_owned(), value);
                    },
                },
                "I Agree To Report My Location *",
                Some(
                    "I voluntary consent to the unencrypted transmission of my node data via MQTT. \
                    (*) The field only makes sense if \"Map Reporting Enabled\" field is set to true.",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(map_report_settings in MqttConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| {
                                v.as_nested()
                                    .get(name_of!(position_precision in MapReportSettings))
                                    .cloned()
                            })
                            .unwrap_or(FormValue::UnsignedInt32(1))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(map_report_settings in MqttConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(map_report_settings in MqttConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(
                                    DEFAULT_MAP_REPORT_SETTINGS.clone(),
                                )))),
                            );
                        }

                        data.get_mut(name_of!(map_report_settings in MqttConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(name_of!(position_precision in MapReportSettings).to_owned(), value);
                    },
                },
                "Position Precision *",
                Some(
                    "0 – location is never sent, 32 – full precision. \
                    (*) The field only makes sense if \"Map Reporting Enabled\" field is set to true",
                ),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=32)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 32"))
                },
            ),
            FormItem::new(
                FormItemKey::Custom {
                    getter: |data| {
                        data.get(name_of!(map_report_settings in MqttConfig))
                            .and_then(|v| v.as_option())
                            .and_then(|v| {
                                v.as_nested()
                                    .get(name_of!(publish_interval_secs in MapReportSettings))
                                    .cloned()
                            })
                            .unwrap_or(FormValue::UnsignedInt32(3600))
                    },
                    setter: |data, value| {
                        if data
                            .get(name_of!(map_report_settings in MqttConfig))
                            .expect("should exists")
                            .as_option()
                            .is_none()
                        {
                            data.insert(
                                name_of!(map_report_settings in MqttConfig).to_owned(),
                                FormValue::Option(Some(Box::new(FormValue::Nested(
                                    DEFAULT_MAP_REPORT_SETTINGS.clone(),
                                )))),
                            );
                        }

                        data.get_mut(name_of!(map_report_settings in MqttConfig))
                            .expect("should exists")
                            .as_option_mut()
                            .expect("should be Some")
                            .as_nested_mut()
                            .insert(name_of!(publish_interval_secs in MapReportSettings).to_owned(), value);
                    },
                },
                "Reporting Interval *",
                Some("(*) The field only makes sense if \"Map Reporting Enabled\" field is set to true."),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 | 3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleSerial,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in SerialConfig)),
                "Serial Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(echo in SerialConfig)),
                "Echo Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rxd in SerialConfig)),
                "RX GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(txd in SerialConfig)),
                "TX GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(baud in SerialConfig)),
                "Baud Rate",
                None,
                FormItemKind::Enum(
                    SerialBaud::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    SerialBaud::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(timeout in SerialConfig)),
                "Timeout",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(mode in SerialConfig)),
                "Mode",
                None,
                FormItemKind::Enum(
                    SerialMode::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    SerialMode::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(override_console_serial_port in SerialConfig)),
                "Override Console Serial Port",
                Some("Overrides the platform's defacto Serial port instance to use with Serial module config settings"),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleExternalNotification,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in ExternalNotificationConfig)),
                "External Notification Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(alert_message in ExternalNotificationConfig)),
                "Alert Message LED",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(alert_message_buzzer in ExternalNotificationConfig)),
                "Alert Message Buzzer",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(alert_message_vibra in ExternalNotificationConfig)),
                "Alert Message Vibra",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(alert_bell in ExternalNotificationConfig)),
                "Alert Bell LED",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(alert_bell_buzzer in ExternalNotificationConfig)),
                "Alert Bell Buzzer",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(alert_bell_vibra in ExternalNotificationConfig)),
                "Alert Bell Vibra",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(output in ExternalNotificationConfig)),
                "Output LED GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=48)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 48"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(output_buzzer in ExternalNotificationConfig)),
                "Output Buzzer GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=48)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 48"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(output_vibra in ExternalNotificationConfig)),
                "Output Vibra GPIO",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=48)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 48"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(output_ms in ExternalNotificationConfig)),
                "Output Duration",
                Some(
                    "When using in On/Off mode, keep the output on for this many milliseconds. \
                    Default 1000ms (1 second).",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("1 second", 1000u32),
                    FormEnumVariant::new("2 seconds", 2000u32),
                    FormEnumVariant::new("3 seconds", 3000u32),
                    FormEnumVariant::new("4 seconds", 4000u32),
                    FormEnumVariant::new("5 seconds", 5000u32),
                    FormEnumVariant::new("10 seconds", 10000u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_string(),
                        1000 => "1 second".to_string(),
                        _ => format!("{} seconds", secs / 1000),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(nag_timeout in ExternalNotificationConfig)),
                "Nag Timeout",
                Some(
                    "The notification will toggle with 'output_ms' for this time of seconds. \
                    Default is 0 which means don't repeat at all. 60 would mean blink and/or \
                    beep for 60 seconds",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("1 second", 1u32),
                    FormEnumVariant::new("5 seconds", 5u32),
                    FormEnumVariant::new("10 seconds", 10u32),
                    FormEnumVariant::new("15 seconds", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_string(),
                        1 => "1 second".to_string(),
                        60 => "1 minute".to_string(),
                        _ => format!("{} seconds", secs),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(use_i2s_as_buzzer in ExternalNotificationConfig)),
                "Use I2C as Buzzer",
                Some(
                    "When true, enables devices with native I2S audio output to use the RTTTL \
                    over speaker like a buzzer T-Watch S3 and T-Deck for example have this capability",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleStoreAndForward,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in StoreForwardConfig)),
                "Store & Forward Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(heartbeat in StoreForwardConfig)),
                "Heartbeat",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(records in StoreForwardConfig)),
                "Number of Records",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(history_return_max in StoreForwardConfig)),
                "History Return Max",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(history_return_window in StoreForwardConfig)),
                "History Return Window",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(is_server in StoreForwardConfig)),
                "Server",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleRangeTest,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in RangeTestConfig)),
                "Range Test Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(sender in RangeTestConfig)),
                "Sender Message Interval",
                Some(
                    "How long to wait between sending sequential test packets. \
                    0 is default which disables sending messages.",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Off", 0u32),
                    FormEnumVariant::new("15 second", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("5 minutes", 5 * 60u32),
                    FormEnumVariant::new("10 minutes", 10 * 60u32),
                    FormEnumVariant::new("15 minutes", 15 * 60u32),
                    FormEnumVariant::new("30 minutes", 30 * 60u32),
                    FormEnumVariant::new("1 hour", 60 * 60u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Off".to_string(),
                        1..60 => format!("{} seconds", secs),
                        60 => "1 minute".to_string(),
                        3600 => "1 hour".to_string(),
                        61..=u32::MAX => format!("{} minutes", secs / 60),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(save in RangeTestConfig)),
                "Save .CSV in storage",
                Some("ESP32 only"),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleTelemetry,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(device_telemetry_enabled in TelemetryConfig)),
                "Send Device Telemetry",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(device_update_interval in TelemetryConfig)),
                "Device Metrics Update Interval",
                None,
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("30 minutes", 1800u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_owned(),
                        1800 => "30 minutes".to_owned(),
                        3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(environment_measurement_enabled in TelemetryConfig)),
                "Environment Metrics Module Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(environment_update_interval in TelemetryConfig)),
                "Environment Metrics Update Interval",
                None,
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("30 minutes", 1800u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_owned(),
                        1800 => "30 minutes".to_owned(),
                        3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(environment_screen_enabled in TelemetryConfig)),
                "Environment Metrics On-Screen Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(environment_display_fahrenheit in TelemetryConfig)),
                "Environment Metrics use Fahrenheit",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(air_quality_enabled in TelemetryConfig)),
                "Air Quality Metrics Module Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(air_quality_interval in TelemetryConfig)),
                "Air Quality Metrics Update Interval",
                None,
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("30 minutes", 1800u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_owned(),
                        1800 => "30 minutes".to_owned(),
                        3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(power_measurement_enabled in TelemetryConfig)),
                "Power Metrics Module Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(power_update_interval in TelemetryConfig)),
                "Power Metrics Update Interval",
                None,
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("30 minutes", 1800u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_owned(),
                        1800 => "30 minutes".to_owned(),
                        3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(power_screen_enabled in TelemetryConfig)),
                "Power Metrics On-Screen Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleCannedMessage,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(rotary1_enabled in CannedMessageConfig)),
                "Rotary Encoder #1 Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(inputbroker_pin_a in CannedMessageConfig)),
                "GPIO Rotary Encoder A Port",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(inputbroker_pin_b in CannedMessageConfig)),
                "GPIO Rotary Encoder B Port",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(inputbroker_pin_press in CannedMessageConfig)),
                "GPIO Rotary Encoder Press Port",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(inputbroker_event_press in CannedMessageConfig)),
                "Input event on Press",
                None,
                FormItemKind::Enum(
                    InputEventChar::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    InputEventChar::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(inputbroker_event_cw in CannedMessageConfig)),
                "Input event on CW",
                None,
                FormItemKind::Enum(
                    InputEventChar::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    InputEventChar::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(inputbroker_event_ccw in CannedMessageConfig)),
                "Input event on CCW",
                None,
                FormItemKind::Enum(
                    InputEventChar::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    InputEventChar::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(updown1_enabled in CannedMessageConfig)),
                "Up/Down/Select Input Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(send_bell in CannedMessageConfig)),
                "Send Bell",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple("messages"),
                "Messages",
                None,
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    (0..=200)
                        .contains(&v.as_string().len())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Max length is 200"))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleNeighborInfo,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in NeighborInfoConfig)),
                "Neighbor Info Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(update_interval in NeighborInfoConfig)),
                "Update Interval",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| format!("{} seconds", v),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(transmit_over_lora in NeighborInfoConfig)),
                "Transmit over LoRa",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleAmbientLighting,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(led_state in AmbientLightingConfig)),
                "LED State",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(current in AmbientLightingConfig)),
                "Current",
                Some("Sets the current for the LED output. Default is 10."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and {}", u32::MAX))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(red in AmbientLightingConfig)),
                "Red",
                Some("Sets the red LED level. Values are 0-255."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=255)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 255"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(green in AmbientLightingConfig)),
                "Green",
                Some("Sets the green LED level. Values are 0-255."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=255)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 255"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(blue in AmbientLightingConfig)),
                "Blue",
                Some("Sets the blue LED level. Values are 0-255."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=255)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 255"))
                },
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleDetectionSensor,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in DetectionSensorConfig)),
                "Detection Sensor Enabled",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(minimum_broadcast_secs in DetectionSensorConfig)),
                "Minimum Broadcast Interval",
                Some(
                    "Interval in seconds of how often we can send a message to the mesh when a trigger event \
                    is detected",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("15 seconds", 15u32),
                    FormEnumVariant::new("30 seconds", 30u32),
                    FormEnumVariant::new("1 minute", 60u32),
                    FormEnumVariant::new("2 minutes", 2 * 60u32),
                    FormEnumVariant::new("5 minutes", 5 * 60u32),
                    FormEnumVariant::new("10 minutes", 10 * 60u32),
                    FormEnumVariant::new("15 minutes", 15 * 60u32),
                    FormEnumVariant::new("30 minutes", 30 * 60u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_owned(),
                        1..60 => format!("{} seconds", secs),
                        60 => "1 minute".to_owned(),
                        61..3600 => format!("{} minutes", secs / 60),
                        3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(state_broadcast_secs in DetectionSensorConfig)),
                "State Broadcast Interval",
                Some(
                    "Interval in seconds of how often we should send a message to the mesh \
                    with the current state regardless of trigger events When set to 0, only \
                    trigger events will be broadcasted Works as a sort of status heartbeat \
                    for peace of mind",
                ),
                FormItemKind::Enum(vec![
                    FormEnumVariant::new("Unset", 0u32),
                    FormEnumVariant::new("15 minutes", 15 * 60u32),
                    FormEnumVariant::new("30 minutes", 30 * 60u32),
                    FormEnumVariant::new("1 hour", 3600u32),
                    FormEnumVariant::new("2 hours", 2 * 3600u32),
                    FormEnumVariant::new("3 hours", 3 * 3600u32),
                    FormEnumVariant::new("4 hours", 4 * 3600u32),
                    FormEnumVariant::new("5 hours", 5 * 3600u32),
                    FormEnumVariant::new("6 hours", 6 * 3600u32),
                    FormEnumVariant::new("12 hours", 12 * 3600u32),
                    FormEnumVariant::new("18 hours", 18 * 3600u32),
                    FormEnumVariant::new("24 hours", 24 * 3600u32),
                    FormEnumVariant::new("36 hours", 36 * 3600u32),
                    FormEnumVariant::new("48 hours", 48 * 3600u32),
                    FormEnumVariant::new("72 hours", 72 * 3600u32),
                ]),
                |v| {
                    let secs = v.as_u32();

                    match secs {
                        0 => "Unset".to_owned(),
                        61..3600 => format!("{} minutes", secs / 60),
                        3600 => "1 hour".to_owned(),
                        _ => format!("{} hours", secs / 3600),
                    }
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(send_bell in DetectionSensorConfig)),
                "Send Bell With Alert Message",
                None,
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(name in DetectionSensorConfig)),
                "Friendly Name",
                Some(
                    "Friendly name used to format message sent to mesh \
                    Example: A name \"Motion\" would result in a message \"Motion detected\"",
                ),
                FormItemKind::InputOfString,
                |v| v.to_string(),
                |v| {
                    (0..=20)
                        .contains(&v.as_string().len())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Max length is 20"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(monitor_pin in DetectionSensorConfig)),
                "GPIO Pin to Monitor",
                None,
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=48)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 48"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(detection_trigger_type in DetectionSensorConfig)),
                "Detection Trigger Type",
                None,
                FormItemKind::Enum(
                    TriggerType::iter()
                        .map(|v| FormEnumVariant::new(v.as_str_name(), v as i32))
                        .collect(),
                ),
                |v| {
                    TriggerType::try_from(v.as_i32()).map(|r| r.as_str_name().to_owned())
                        .unwrap_or("?".to_owned())
                },
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(use_pullup in DetectionSensorConfig)),
                "Use INPUT_PULLUP Mode",
                Some(
                    "Whether or not use INPUT_PULLUP mode for GPIO pin. Only applicable if the board \
                    uses pull-up resistors on the pin",
                ),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms.insert(
        FormId::ModuleTrafficManagement,
        Vec::from([
            FormItem::new(
                FormItemKey::Simple(name_of!(enabled in TrafficManagementConfig)),
                "Traffic Management Enabled",
                Some("Master enable for traffic management module."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(position_dedup_enabled in TrafficManagementConfig)),
                "Position Deduplication Enabled",
                Some("Drop redundant position broadcasts from the same node."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(position_precision_bits in TrafficManagementConfig)),
                "Position Precision Bits",
                Some("Number of bits of precision for position deduplication (0-32)."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=32)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 32"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(position_min_interval_secs in TrafficManagementConfig)),
                "Min Position Interval",
                Some("Minimum interval in seconds between position updates from the same node."),
                FormItemKind::InputOfUnsignedInt32,
                |v| format!("{} seconds", v.as_u32()),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be a valid number of seconds"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(nodeinfo_direct_response in TrafficManagementConfig)),
                "NodeInfo Direct Response",
                Some("Enable direct responses to NodeInfo requests from local cache."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(nodeinfo_direct_response_max_hops in TrafficManagementConfig)),
                "NodeInfo Response Max Hops",
                Some("Minimum hop distance from requestor before responding to NodeInfo requests."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=3)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be between 0 and 255"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rate_limit_enabled in TrafficManagementConfig)),
                "Rate Limit Enabled",
                Some("Enable per-node rate limiting to throttle chatty nodes."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rate_limit_window_secs in TrafficManagementConfig)),
                "Rate Limit Window",
                Some("Time window in seconds for rate limiting calculations."),
                FormItemKind::InputOfUnsignedInt32,
                |v| format!("{} seconds", v.as_u32()),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be a valid number of seconds"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(rate_limit_max_packets in TrafficManagementConfig)),
                "Rate Limit Max Packets",
                Some("Maximum packets allowed per node within the rate limit window."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be a valid packet count"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(drop_unknown_enabled in TrafficManagementConfig)),
                "Drop Unknown Packets",
                Some("Enable dropping of unknown/undecryptable packets per rate limit window."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(unknown_packet_threshold in TrafficManagementConfig)),
                "Unknown Packet Threshold",
                Some("Number of unknown packets before dropping from a node."),
                FormItemKind::InputOfUnsignedInt32,
                |v| v.to_string(),
                |v| {
                    (0..=u32::MAX)
                        .contains(&v.as_u32())
                        .then_some(())
                        .ok_or(anyhow::anyhow!("Must be a valid packet count"))
                },
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(exhaust_hop_telemetry in TrafficManagementConfig)),
                "Exhaust Hop Telemetry",
                Some("Set hop_limit to 0 for relayed telemetry broadcasts (own packets unaffected)."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(exhaust_hop_position in TrafficManagementConfig)),
                "Exhaust Hop Position",
                Some("Set hop_limit to 0 for relayed position broadcasts (own packets unaffected)."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
            FormItem::new(
                FormItemKey::Simple(name_of!(router_preserve_hops in TrafficManagementConfig)),
                "Router Preserve Hops",
                Some("Preserve hop_limit for router-to-router traffic."),
                FormItemKind::Switch,
                |v| v.to_string(),
                |_| Ok(()),
            ),
        ]),
    );

    forms
}
