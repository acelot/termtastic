use crate::repository::domain::telemetry::Telemetry;
use crate::types::NodeTelemetry;
use meshtastic::protobufs::telemetry;

impl TryFrom<NodeTelemetry> for Telemetry {
    type Error = anyhow::Error;

    fn try_from(value: NodeTelemetry) -> Result<Self, Self::Error> {
        let (kind, data) = match value.variant {
            telemetry::Variant::DeviceMetrics(v) => ("device_metrics", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::EnvironmentMetrics(v) => ("environment_metrics", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::AirQualityMetrics(v) => ("air_quality_metrics", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::PowerMetrics(v) => ("power_metrics", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::LocalStats(v) => ("local_stats", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::HealthMetrics(v) => ("health_metrics", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::HostMetrics(v) => ("host_metrics", serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::TrafficManagementStats(v) => {
                ("traffic_management_stats", serde_sqlite_jsonb::to_vec(&v)?)
            }
        };

        Ok(Self {
            id: None,
            node_key: value.node_key,
            datetime: value.datetime,
            kind: kind.to_owned(),
            data: data.to_vec(),
        })
    }
}

impl TryFrom<&Telemetry> for NodeTelemetry {
    type Error = anyhow::Error;

    fn try_from(value: &Telemetry) -> Result<Self, Self::Error> {
        let slice = value.data.as_slice();

        let variant = match value.kind.as_str() {
            "device_metrics" => telemetry::Variant::DeviceMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::DeviceMetrics,
            >(slice)?),
            "environment_metrics" => telemetry::Variant::EnvironmentMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::EnvironmentMetrics,
            >(slice)?),
            "air_quality_metrics" => telemetry::Variant::AirQualityMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::AirQualityMetrics,
            >(slice)?),
            "power_metrics" => telemetry::Variant::PowerMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::PowerMetrics,
            >(slice)?),
            "local_stats" => telemetry::Variant::LocalStats(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::LocalStats,
            >(slice)?),
            "health_metrics" => telemetry::Variant::HealthMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::HealthMetrics,
            >(slice)?),
            "host_metrics" => telemetry::Variant::HostMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::HostMetrics,
            >(slice)?),
            "traffic_management_stats" => telemetry::Variant::TrafficManagementStats(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::TrafficManagementStats,
            >(slice)?),
            _ => anyhow::bail!("unknown telemetry kind: {}", value.kind),
        };

        Ok(Self {
            node_key: value.node_key,
            datetime: value.datetime,
            variant,
        })
    }
}
