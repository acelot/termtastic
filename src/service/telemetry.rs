use crate::state::State;
use crate::types::TelemetryItem;
use crate::ui::helpers::humanize_uptime;

macro_rules! power_metrics {
    ($items:expr, $power_metrics:expr, $($ch:literal), * $(,)?) => {
        $(
            paste::paste! {
                $items.push(TelemetryItem::item(
                    concat!("ch", $ch, "_voltage"),
                    $power_metrics.data.[<ch $ch _voltage>],
                    $power_metrics.data.[<ch $ch _voltage>].map(|v| format!("{:.1}V", v)),
                ));

                $items.push(TelemetryItem::item(
                    concat!("ch", $ch, " current"),
                    $power_metrics.data.[<ch $ch _current>],
                    $power_metrics.data.[<ch $ch _current>].and_then(|v| Some(format!("{:.1}A", v))),
                ));
            }
        )*
    };
}

pub fn update_nodeinfo_telemetry(state: &mut State) -> bool {
    let Some(node_key) = state.nodeinfo else {
        return false;
    };

    let Some(last_telemetry) = &state.nodes_last_telemetry.get(&node_key) else {
        return false;
    };

    let mut items: Vec<TelemetryItem> = Vec::new();

    // device metrics
    if let Some(device_metrics) = &last_telemetry.device_metrics {
        items.push(TelemetryItem::group(
            "Device Metrics",
            device_metrics.datetime,
            serde_json::to_string_pretty(&device_metrics.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "battery level",
            device_metrics.data.battery_level,
            device_metrics.data.battery_level.map(|v| {
                if v > 100 {
                    "\u{1F50C} PWR".to_owned()
                } else {
                    format!("{}%", v)
                }
            }),
        ));

        items.push(TelemetryItem::item(
            "voltage",
            device_metrics.data.voltage,
            device_metrics.data.voltage.map(|v| format!("{:.1}V", v)),
        ));

        items.push(TelemetryItem::item(
            "air util tx",
            device_metrics.data.air_util_tx,
            device_metrics.data.air_util_tx.map(|v| format!("{:.2}%", v)),
        ));

        items.push(TelemetryItem::item(
            "channel util",
            device_metrics.data.channel_utilization,
            device_metrics
                .data
                .channel_utilization.map(|v| format!("{:.2}%", v)),
        ));

        items.push(TelemetryItem::item(
            "uptime",
            device_metrics.data.uptime_seconds,
            device_metrics
                .data
                .uptime_seconds.map(humanize_uptime),
        ));
    }

    // environment metrics
    if let Some(environment_metrics) = &last_telemetry.environment_metrics {
        items.push(TelemetryItem::group(
            "Environment Metrics",
            environment_metrics.datetime,
            serde_json::to_string_pretty(&environment_metrics.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "temperature",
            environment_metrics.data.temperature,
            environment_metrics
                .data
                .temperature.map(|v| format!("{:.1}°C", v)),
        ));

        items.push(TelemetryItem::item(
            "relative humidity",
            environment_metrics.data.relative_humidity,
            environment_metrics
                .data
                .relative_humidity.map(|v| format!("{:.1}%", v)),
        ));

        items.push(TelemetryItem::item(
            "barometric pressure",
            environment_metrics.data.barometric_pressure,
            environment_metrics
                .data
                .barometric_pressure.map(|v| format!("{:.1}hPA", v)),
        ));

        items.push(TelemetryItem::item(
            "gas resistance",
            environment_metrics.data.gas_resistance,
            environment_metrics
                .data
                .gas_resistance.map(|v| format!("{:.1}MOhm", v)),
        ));

        items.push(TelemetryItem::item(
            "current",
            environment_metrics.data.current,
            environment_metrics
                .data
                .current.map(|v| format!("{:.1}A", v)),
        ));

        items.push(TelemetryItem::item(
            "voltage",
            environment_metrics.data.voltage,
            environment_metrics
                .data
                .voltage.map(|v| format!("{:.1}V", v)),
        ));

        items.push(TelemetryItem::item(
            "IAQ",
            environment_metrics.data.iaq,
            environment_metrics.data.iaq.map(|v| v),
        ));

        items.push(TelemetryItem::item(
            "distance",
            environment_metrics.data.distance,
            environment_metrics
                .data
                .distance.map(|v| format!("{:.3}mm", v)),
        ));

        items.push(TelemetryItem::item(
            "lux",
            environment_metrics.data.lux,
            environment_metrics.data.lux.map(|v| format!("{:.2}", v)),
        ));

        items.push(TelemetryItem::item(
            "white lux",
            environment_metrics.data.white_lux,
            environment_metrics
                .data
                .white_lux.map(|v| format!("{:.2}", v)),
        ));

        items.push(TelemetryItem::item(
            "infrared lux",
            environment_metrics.data.ir_lux,
            environment_metrics.data.ir_lux.map(|v| format!("{:.2}", v)),
        ));

        items.push(TelemetryItem::item(
            "ultraviolet lux",
            environment_metrics.data.uv_lux,
            environment_metrics.data.uv_lux.map(|v| format!("{:.2}", v)),
        ));

        items.push(TelemetryItem::item(
            "wind direction",
            environment_metrics.data.wind_direction,
            environment_metrics
                .data
                .wind_direction.map(|v| format!("{}°", v)),
        ));

        items.push(TelemetryItem::item(
            "wind speed",
            environment_metrics.data.wind_speed,
            environment_metrics
                .data
                .wind_speed.map(|v| format!("{:.2}m/s", v)),
        ));

        items.push(TelemetryItem::item(
            "wind gust",
            environment_metrics.data.wind_gust,
            environment_metrics
                .data
                .wind_gust.map(|v| format!("{:.2}m/s", v)),
        ));

        items.push(TelemetryItem::item(
            "wind lull",
            environment_metrics.data.wind_lull,
            environment_metrics
                .data
                .wind_lull.map(|v| format!("{:.2}m/s", v)),
        ));

        items.push(TelemetryItem::item(
            "weight",
            environment_metrics.data.weight,
            environment_metrics
                .data
                .weight.map(|v| format!("{:.3}kg", v)),
        ));

        items.push(TelemetryItem::item(
            "radiation",
            environment_metrics.data.radiation,
            environment_metrics
                .data
                .radiation.map(|v| format!("{:.3}µR/h", v)),
        ));

        items.push(TelemetryItem::item(
            "rainfall 1h",
            environment_metrics.data.rainfall_1h,
            environment_metrics
                .data
                .rainfall_1h.map(|v| format!("{:.1}mm", v)),
        ));

        items.push(TelemetryItem::item(
            "rainfall 24h",
            environment_metrics.data.rainfall_24h,
            environment_metrics
                .data
                .rainfall_24h.map(|v| format!("{:.1}mm", v)),
        ));

        items.push(TelemetryItem::item(
            "soil moisture",
            environment_metrics.data.soil_moisture,
            environment_metrics
                .data
                .soil_moisture.map(|v| format!("{}%", v)),
        ));

        items.push(TelemetryItem::item(
            "soil temperature",
            environment_metrics.data.soil_temperature,
            environment_metrics
                .data
                .soil_temperature.map(|v| format!("{:.1}°C", v)),
        ));
    }

    // air quality metrics
    if let Some(air_quality_metrics) = &last_telemetry.air_quality_metrics {
        items.push(TelemetryItem::group(
            "Air Quality Metrics",
            air_quality_metrics.datetime,
            serde_json::to_string_pretty(&air_quality_metrics.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "PM1.0 standard",
            air_quality_metrics.data.pm10_standard,
            air_quality_metrics
                .data
                .pm10_standard.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "PM2.5 standard",
            air_quality_metrics.data.pm25_standard,
            air_quality_metrics
                .data
                .pm25_standard.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "PM10 standard",
            air_quality_metrics.data.pm100_standard,
            air_quality_metrics
                .data
                .pm100_standard.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "PM1.0 environmental",
            air_quality_metrics.data.pm10_environmental,
            air_quality_metrics
                .data
                .pm10_environmental.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "PM2.5 environmental",
            air_quality_metrics.data.pm25_environmental,
            air_quality_metrics
                .data
                .pm25_environmental.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "PM10 environmental",
            air_quality_metrics.data.pm100_environmental,
            air_quality_metrics
                .data
                .pm100_environmental.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "0.3µm particles",
            air_quality_metrics.data.particles_03um,
            air_quality_metrics
                .data
                .particles_03um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "0.5µm particles",
            air_quality_metrics.data.particles_05um,
            air_quality_metrics
                .data
                .particles_05um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "1.0µm particles",
            air_quality_metrics.data.particles_10um,
            air_quality_metrics
                .data
                .particles_10um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "2.5µm particles",
            air_quality_metrics.data.particles_25um,
            air_quality_metrics
                .data
                .particles_25um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "5.0µm particles",
            air_quality_metrics.data.particles_50um,
            air_quality_metrics
                .data
                .particles_50um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "10.0µm particles",
            air_quality_metrics.data.particles_100um,
            air_quality_metrics
                .data
                .particles_100um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "CO2",
            air_quality_metrics.data.co2,
            air_quality_metrics.data.co2.map(|v| format!("{}ppm", v)),
        ));

        items.push(TelemetryItem::item(
            "CO2 temperature",
            air_quality_metrics.data.co2_temperature,
            air_quality_metrics
                .data
                .co2_temperature.map(|v| format!("{:.1}°C", v)),
        ));

        items.push(TelemetryItem::item(
            "CO2 humidity",
            air_quality_metrics.data.co2_humidity,
            air_quality_metrics
                .data
                .co2_humidity.map(|v| format!("{:.1}%", v)),
        ));

        items.push(TelemetryItem::item(
            "formaldehyde",
            air_quality_metrics.data.form_formaldehyde,
            air_quality_metrics
                .data
                .form_formaldehyde.map(|v| format!("{:.1}ppb", v)),
        ));

        items.push(TelemetryItem::item(
            "formaldehyde humidity",
            air_quality_metrics.data.form_humidity,
            air_quality_metrics
                .data
                .form_humidity.map(|v| format!("{:.1}%RH", v)),
        ));

        items.push(TelemetryItem::item(
            "formaldehyde temperature",
            air_quality_metrics.data.form_temperature,
            air_quality_metrics
                .data
                .form_temperature.map(|v| format!("{:.1}°C", v)),
        ));

        items.push(TelemetryItem::item(
            "PM4.0 standard",
            air_quality_metrics.data.pm40_standard,
            air_quality_metrics
                .data
                .pm40_standard.map(|v| format!("{}µg/m³", v)),
        ));

        items.push(TelemetryItem::item(
            "4.0µm particles",
            air_quality_metrics.data.particles_40um,
            air_quality_metrics
                .data
                .particles_40um.map(|v| format!("{}/0.1L", v)),
        ));

        items.push(TelemetryItem::item(
            "PM temperature",
            air_quality_metrics.data.pm_temperature,
            air_quality_metrics
                .data
                .pm_temperature.map(|v| format!("{:.1}°C", v)),
        ));

        items.push(TelemetryItem::item(
            "PM humidity",
            air_quality_metrics.data.pm_humidity,
            air_quality_metrics
                .data
                .pm_humidity.map(|v| format!("{:.1}%", v)),
        ));

        items.push(TelemetryItem::item(
            "PM VOC index",
            air_quality_metrics.data.pm_voc_idx,
            air_quality_metrics
                .data
                .pm_voc_idx.map(|v| format!("{:.2}", v)),
        ));

        items.push(TelemetryItem::item(
            "PM NOx index",
            air_quality_metrics.data.pm_nox_idx,
            air_quality_metrics
                .data
                .pm_nox_idx.map(|v| format!("{:.2}", v)),
        ));

        items.push(TelemetryItem::item(
            "typical particle size",
            air_quality_metrics.data.particles_tps,
            air_quality_metrics
                .data
                .particles_tps.map(|v| format!("{:.2}µm", v)),
        ));
    }

    // host metrics
    if let Some(host_metrics) = &last_telemetry.host_metrics {
        items.push(TelemetryItem::group(
            "Host Metrics",
            host_metrics.datetime,
            serde_json::to_string_pretty(&host_metrics.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "uptime",
            Some(host_metrics.data.uptime_seconds),
            Some(humanize_uptime(host_metrics.data.uptime_seconds)),
        ));

        items.push(TelemetryItem::item(
            "free memory",
            Some(host_metrics.data.freemem_bytes),
            Some(format!("{}MB", host_metrics.data.freemem_bytes / 1024)),
        ));

        items.push(TelemetryItem::item(
            "disk 1 free space",
            Some(host_metrics.data.diskfree1_bytes),
            Some(format!("{}MB", host_metrics.data.diskfree1_bytes / 1024)),
        ));

        items.push(TelemetryItem::item(
            "disk 2 free space",
            host_metrics.data.diskfree2_bytes,
            host_metrics
                .data
                .diskfree2_bytes.map(|b| format!("{}MB", b / 1024)),
        ));

        items.push(TelemetryItem::item(
            "disk 3 free space",
            host_metrics.data.diskfree3_bytes,
            host_metrics
                .data
                .diskfree3_bytes.map(|b| format!("{}MB", b / 1024)),
        ));

        items.push(TelemetryItem::item(
            "load 1 min",
            Some(host_metrics.data.load1),
            Some(host_metrics.data.load1),
        ));

        items.push(TelemetryItem::item(
            "load 5 min",
            Some(host_metrics.data.load5),
            Some(host_metrics.data.load5),
        ));

        items.push(TelemetryItem::item(
            "load 15 min",
            Some(host_metrics.data.load15),
            Some(host_metrics.data.load15),
        ));

        items.push(TelemetryItem::item(
            "user string",
            host_metrics.data.user_string.as_ref(),
            host_metrics.data.user_string.as_ref(),
        ));
    }

    // power metrics
    if let Some(power_metrics) = &last_telemetry.power_metrics {
        items.push(TelemetryItem::group(
            "Power Metrics",
            power_metrics.datetime,
            serde_json::to_string_pretty(&power_metrics.data).unwrap_or("serialize failed".to_owned()),
        ));

        power_metrics!(items, power_metrics, "1", "2", "3", "4", "5", "6", "7", "8");
    }

    // local stats
    if let Some(local_stats) = &last_telemetry.local_stats {
        items.push(TelemetryItem::group(
            "Local Stats",
            local_stats.datetime,
            serde_json::to_string_pretty(&local_stats.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "uptime",
            Some(local_stats.data.uptime_seconds),
            Some(humanize_uptime(local_stats.data.uptime_seconds)),
        ));

        items.push(TelemetryItem::item(
            "channel utilization",
            Some(local_stats.data.channel_utilization),
            Some(format!("{:.2}%", local_stats.data.channel_utilization)),
        ));

        items.push(TelemetryItem::item(
            "air util tx",
            Some(local_stats.data.air_util_tx),
            Some(format!("{:.2}%", local_stats.data.air_util_tx)),
        ));

        items.push(TelemetryItem::item(
            "packets tx",
            Some(local_stats.data.num_packets_tx),
            Some(format!("{}", local_stats.data.num_packets_tx)),
        ));

        items.push(TelemetryItem::item(
            "packets rx",
            Some(local_stats.data.num_packets_rx),
            Some(format!("{}", local_stats.data.num_packets_rx)),
        ));

        items.push(TelemetryItem::item(
            "packets rx bad",
            Some(local_stats.data.num_packets_rx_bad),
            Some(format!("{}", local_stats.data.num_packets_rx_bad)),
        ));

        items.push(TelemetryItem::item(
            "online nodes",
            Some(local_stats.data.num_online_nodes),
            Some(format!("{}", local_stats.data.num_online_nodes)),
        ));

        items.push(TelemetryItem::item(
            "total nodes",
            Some(local_stats.data.num_total_nodes),
            Some(format!("{}", local_stats.data.num_total_nodes)),
        ));

        items.push(TelemetryItem::item(
            "rx dupe",
            Some(local_stats.data.num_rx_dupe),
            Some(format!("{}", local_stats.data.num_rx_dupe)),
        ));

        items.push(TelemetryItem::item(
            "tx relay",
            Some(local_stats.data.num_tx_relay),
            Some(format!("{}", local_stats.data.num_tx_relay)),
        ));

        items.push(TelemetryItem::item(
            "tx relay canceled",
            Some(local_stats.data.num_tx_relay_canceled),
            Some(format!("{}", local_stats.data.num_tx_relay_canceled)),
        ));

        items.push(TelemetryItem::item(
            "heap total",
            Some(local_stats.data.heap_total_bytes),
            Some(format!("{}KB", local_stats.data.heap_total_bytes / 1024)),
        ));

        items.push(TelemetryItem::item(
            "heap free",
            Some(local_stats.data.heap_free_bytes),
            Some(format!("{}KB", local_stats.data.heap_free_bytes / 1024)),
        ));

        items.push(TelemetryItem::item(
            "tx dropped",
            Some(local_stats.data.num_tx_dropped),
            Some(format!("{}", local_stats.data.num_tx_dropped)),
        ));

        items.push(TelemetryItem::item(
            "noise floor",
            Some(local_stats.data.noise_floor),
            Some(format!("{}dBm", local_stats.data.noise_floor)),
        ));
    }

    // health metrics
    if let Some(health_metrics) = &last_telemetry.health_metrics {
        items.push(TelemetryItem::group(
            "Health Metrics",
            health_metrics.datetime,
            serde_json::to_string_pretty(&health_metrics.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "heart rate",
            health_metrics.data.heart_bpm,
            health_metrics.data.heart_bpm.map(|v| format!("{}bpm", v)),
        ));

        items.push(TelemetryItem::item(
            "SpO2",
            health_metrics.data.sp_o2,
            health_metrics.data.sp_o2.map(|v| format!("{}%", v)),
        ));

        items.push(TelemetryItem::item(
            "body temperature",
            health_metrics.data.temperature,
            health_metrics
                .data
                .temperature.map(|v| format!("{:.1}°C", v)),
        ));
    }

    // traffic management stats
    if let Some(traffic_management_stats) = &last_telemetry.traffic_management_stats {
        items.push(TelemetryItem::group(
            "Traffic Management Stats",
            traffic_management_stats.datetime,
            serde_json::to_string_pretty(&traffic_management_stats.data).unwrap_or("serialize failed".to_owned()),
        ));

        items.push(TelemetryItem::item(
            "packets inspected",
            Some(traffic_management_stats.data.packets_inspected),
            Some(format!("{}", traffic_management_stats.data.packets_inspected)),
        ));

        items.push(TelemetryItem::item(
            "position dedup drops",
            Some(traffic_management_stats.data.position_dedup_drops),
            Some(format!("{}", traffic_management_stats.data.position_dedup_drops)),
        ));

        items.push(TelemetryItem::item(
            "nodeinfo cache hits",
            Some(traffic_management_stats.data.nodeinfo_cache_hits),
            Some(format!("{}", traffic_management_stats.data.nodeinfo_cache_hits)),
        ));

        items.push(TelemetryItem::item(
            "rate limit drops",
            Some(traffic_management_stats.data.rate_limit_drops),
            Some(format!("{}", traffic_management_stats.data.rate_limit_drops)),
        ));

        items.push(TelemetryItem::item(
            "unknown packet drops",
            Some(traffic_management_stats.data.unknown_packet_drops),
            Some(format!("{}", traffic_management_stats.data.unknown_packet_drops)),
        ));

        items.push(TelemetryItem::item(
            "hop exhausted packets",
            Some(traffic_management_stats.data.hop_exhausted_packets),
            Some(format!("{}", traffic_management_stats.data.hop_exhausted_packets)),
        ));

        items.push(TelemetryItem::item(
            "router hops preserved",
            Some(traffic_management_stats.data.router_hops_preserved),
            Some(format!("{}", traffic_management_stats.data.router_hops_preserved)),
        ));
    }

    state.nodeinfo_telemetry = items
        .into_iter()
        .filter(|item| match item {
            TelemetryItem::Group { .. } => true,
            TelemetryItem::Item { value, .. } => value.is_some(),
        })
        .collect();

    true
}
