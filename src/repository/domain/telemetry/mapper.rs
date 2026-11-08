use crate::repository::domain::telemetry::Telemetry;
use crate::types::{NodeTelemetry, TelemetryKind};
use meshtastic::protobufs::telemetry;

impl TryFrom<NodeTelemetry> for Telemetry {
    type Error = anyhow::Error;

    fn try_from(value: NodeTelemetry) -> Result<Self, Self::Error> {
        let (kind, data) = match value.variant {
            telemetry::Variant::DeviceMetrics(v) => (TelemetryKind::DeviceMetrics, serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::EnvironmentMetrics(v) => {
                (TelemetryKind::EnvironmentMetrics, serde_sqlite_jsonb::to_vec(&v)?)
            }
            telemetry::Variant::AirQualityMetrics(v) => {
                (TelemetryKind::AirQualityMetrics, serde_sqlite_jsonb::to_vec(&v)?)
            }
            telemetry::Variant::PowerMetrics(v) => (TelemetryKind::PowerMetrics, serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::LocalStats(v) => (TelemetryKind::LocalStats, serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::HealthMetrics(v) => (TelemetryKind::HealthMetrics, serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::HostMetrics(v) => (TelemetryKind::HostMetrics, serde_sqlite_jsonb::to_vec(&v)?),
            telemetry::Variant::TrafficManagementStats(v) => {
                (TelemetryKind::TrafficManagementStats, serde_sqlite_jsonb::to_vec(&v)?)
            }
        };

        Ok(Self {
            id: None,
            node_key: value.node_key,
            datetime: value.datetime,
            kind,
            data: data.to_vec(),
        })
    }
}

impl TryFrom<&Telemetry> for NodeTelemetry {
    type Error = anyhow::Error;

    fn try_from(value: &Telemetry) -> Result<Self, Self::Error> {
        let slice = value.data.as_slice();

        let variant = match value.kind {
            TelemetryKind::DeviceMetrics => telemetry::Variant::DeviceMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::DeviceMetrics,
            >(slice)?),
            TelemetryKind::EnvironmentMetrics => {
                telemetry::Variant::EnvironmentMetrics(serde_sqlite_jsonb::from_slice::<
                    meshtastic::protobufs::EnvironmentMetrics,
                >(slice)?)
            }
            TelemetryKind::AirQualityMetrics => {
                telemetry::Variant::AirQualityMetrics(serde_sqlite_jsonb::from_slice::<
                    meshtastic::protobufs::AirQualityMetrics,
                >(slice)?)
            }
            TelemetryKind::PowerMetrics => telemetry::Variant::PowerMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::PowerMetrics,
            >(slice)?),
            TelemetryKind::LocalStats => telemetry::Variant::LocalStats(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::LocalStats,
            >(slice)?),
            TelemetryKind::HealthMetrics => telemetry::Variant::HealthMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::HealthMetrics,
            >(slice)?),
            TelemetryKind::HostMetrics => telemetry::Variant::HostMetrics(serde_sqlite_jsonb::from_slice::<
                meshtastic::protobufs::HostMetrics,
            >(slice)?),
            TelemetryKind::TrafficManagementStats => {
                telemetry::Variant::TrafficManagementStats(serde_sqlite_jsonb::from_slice::<
                    meshtastic::protobufs::TrafficManagementStats,
                >(slice)?)
            }
        };

        Ok(Self {
            node_key: value.node_key,
            datetime: value.datetime,
            variant,
        })
    }
}
