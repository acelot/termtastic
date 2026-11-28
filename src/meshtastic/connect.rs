use btleplug::api::BDAddr;
use hostaddr::HostAddr;
use meshtastic::api::{ConnectedStreamApi, StreamApi};
use meshtastic::protobufs::FromRadio;
use meshtastic::utils;
use std::time::Duration;
use tokio::sync::mpsc;

const MESHTASTIC_DEFAULT_TCP_PORT: u16 = 4403;

fn generate_want_config_id() -> u32 {
    // Special want_config_id values, defined in firmware src/mesh/PhoneApi.h
    const SPECIAL_NONCE_ONLY_CONFIG: u32 = 69420;
    const SPECIAL_NONCE_ONLY_NODES: u32 = 69421;
    let mut random_nonce = utils::generate_rand_id();
    let mut attempt = 0;
    while random_nonce == SPECIAL_NONCE_ONLY_CONFIG || random_nonce == SPECIAL_NONCE_ONLY_NODES {
        random_nonce = utils::generate_rand_id();
        attempt += 1;
        if attempt > 5 {
            // Very unlikely to happen for random id generator
            panic!("meshtastic::utils::generate_rand_id generates non-random special values");
        }
    }
    random_nonce
}

pub async fn connect_via_tcp(
    address: HostAddr<String>,
) -> anyhow::Result<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)> {
    let stream_handle = utils::stream::build_tcp_stream(if !address.has_port() {
        address.with_port(MESHTASTIC_DEFAULT_TCP_PORT).to_string()
    } else {
        address.to_string()
    })
    .await?;

    let stream_api = StreamApi::new();
    let (from_radio_receiver, connected_stream_api) = stream_api.connect(stream_handle).await;

    let connected_stream_api = connected_stream_api.configure(generate_want_config_id()).await?;

    Ok((from_radio_receiver, connected_stream_api))
}

pub async fn connect_via_ble(
    address: BDAddr,
    name: Option<String>,
) -> anyhow::Result<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)> {
    let ble_id = match name {
        Some(n) => utils::stream::BleId::Name(n),
        None => utils::stream::BleId::MacAddress(address),
    };

    let stream_handle = utils::stream::build_ble_stream(ble_id, Duration::from_secs(5)).await?;

    let stream_api = StreamApi::new();
    let (from_radio_receiver, connected_stream_api) = stream_api.connect(stream_handle).await;

    let connected_stream_api = connected_stream_api.configure(generate_want_config_id()).await?;

    Ok((from_radio_receiver, connected_stream_api))
}

pub async fn connect_via_serial(
    address: String,
) -> anyhow::Result<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)> {
    let stream_handle = utils::stream::build_serial_stream(address, None, None, None)?;

    let stream_api = StreamApi::new();
    let (from_radio_receiver, connected_stream_api) = stream_api.connect(stream_handle).await;

    let connected_stream_api = connected_stream_api.configure(generate_want_config_id()).await?;

    Ok((from_radio_receiver, connected_stream_api))
}
