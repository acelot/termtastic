use std::time::Duration;

use hostaddr::HostAddr;
use meshtastic::api::{ConnectedStreamApi, StreamApi};
use meshtastic::protobufs::FromRadio;
use meshtastic::utils;
use tokio::sync::mpsc;

pub async fn connect_via_tcp(
    hostaddr: HostAddr<String>,
) -> anyhow::Result<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)> {
    let stream_handle = utils::stream::build_tcp_stream(hostaddr.to_string()).await?;

    let stream_api = StreamApi::new();
    let (from_radio_receiver, connected_stream_api) = stream_api.connect(stream_handle).await;

    let connected_stream_api = connected_stream_api
        .configure(utils::generate_rand_id())
        .await?;

    Ok((from_radio_receiver, connected_stream_api))
}

pub async fn connect_via_ble(
    address: String,
) -> anyhow::Result<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)> {
    let stream_handle = utils::stream::build_ble_stream(
        &utils::stream::BleId::from_name(&address),
        Duration::from_secs(5),
    )
    .await?;

    let stream_api = StreamApi::new();
    let (from_radio_receiver, connected_stream_api) = stream_api.connect(stream_handle).await;

    let connected_stream_api = connected_stream_api
        .configure(utils::generate_rand_id())
        .await?;

    Ok((from_radio_receiver, connected_stream_api))
}

pub async fn connect_via_serial(
    address: String,
) -> anyhow::Result<(mpsc::UnboundedReceiver<FromRadio>, ConnectedStreamApi)> {
    let stream_handle = utils::stream::build_serial_stream(address, None, None, None)?;

    let stream_api = StreamApi::new();
    let (from_radio_receiver, connected_stream_api) = stream_api.connect(stream_handle).await;

    let connected_stream_api = connected_stream_api
        .configure(utils::generate_rand_id())
        .await?;

    Ok((from_radio_receiver, connected_stream_api))
}
