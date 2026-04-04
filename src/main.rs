mod log2state;
mod meshtastic;
mod service;
mod state;
mod types;
mod ui;

use std::time::Duration;

use tokio::sync::broadcast;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_unwrap::ResultExt;

use crate::{
    log2state::LogToState,
    meshtastic::MeshtasticService,
    service::{ChatService, ConfigService, ConnectionService, NodesService, UiService},
    state::{State, Store},
    types::AppEvent,
    ui::Ui,
};

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let (store, state_action_tx, state_rx) = Store::new(State::default());

    let log_filter = tracing_subscriber::filter::Targets::new()
        .with_default(tracing::Level::DEBUG)
        .with_target("meshtastic", tracing::level_filters::LevelFilter::OFF);

    let (file_writer, _file_writer_guard) = tracing_appender::non_blocking(
        tracing_appender::rolling::daily("logs", format!("{}.log", APP_NAME)),
    );
    let file_logger_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false);

    let log_to_state_layer = LogToState::new(state_action_tx.clone());

    tracing_subscriber::registry()
        .with(log_filter)
        .with(file_logger_layer)
        .with(log_to_state_layer)
        .init();

    tracing::info!("application started");

    let (meshtastic_service, meshtastic_command_tx, meshtastic_event_rx) = MeshtasticService::new();
    let (event_tx, event_rx) = broadcast::channel::<AppEvent>(100);

    let config_service = ConfigService::new(
        event_rx.resubscribe(),
        state_rx.clone(),
        state_action_tx.clone(),
    );

    let ui_service = UiService::new(
        event_tx.clone(),
        event_rx.resubscribe(),
        state_rx.clone(),
        state_action_tx.clone(),
        meshtastic_command_tx.clone(),
        meshtastic_event_rx.resubscribe(),
    );

    let nodes_service = NodesService::new(
        event_tx.clone(),
        event_rx.resubscribe(),
        state_rx.clone(),
        state_action_tx.clone(),
        meshtastic_command_tx.clone(),
        meshtastic_event_rx.resubscribe(),
    );

    let connection_service = ConnectionService::new(
        event_tx.clone(),
        event_rx.resubscribe(),
        state_rx.clone(),
        state_action_tx.clone(),
        meshtastic_command_tx.clone(),
        meshtastic_event_rx.resubscribe(),
    );

    let chat_service = ChatService::new(
        event_rx.resubscribe(),
        state_rx.clone(),
        state_action_tx.clone(),
        meshtastic_command_tx.clone(),
        meshtastic_event_rx.resubscribe(),
    );

    let event_tx_clone = event_tx.clone();
    let state_action_tx_clone = state_action_tx.clone();

    event_tx_clone
        .send(AppEvent::InitializationRequested)
        .expect_or_log("InitializationRequested event should be sent");

    Toplevel::new(async |s: &mut SubsystemHandle| {
        s.start(SubsystemBuilder::new(
            "ConfigService",
            async |subsys: &mut SubsystemHandle| config_service.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "Store",
            async |subsys: &mut SubsystemHandle| store.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "UiService",
            async |subsys: &mut SubsystemHandle| ui_service.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "NodesService",
            async |subsys: &mut SubsystemHandle| nodes_service.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "ConnectionService",
            async |subsys: &mut SubsystemHandle| connection_service.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "ChatService",
            async |subsys: &mut SubsystemHandle| chat_service.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "MeshtasticService",
            async |subsys: &mut SubsystemHandle| meshtastic_service.run(subsys).await,
        ));

        s.start(SubsystemBuilder::new(
            "UI",
            async |subsys: &mut SubsystemHandle| {
                Ui::new(state_rx, state_action_tx_clone, event_tx_clone)
                    .run(subsys)
                    .await
            },
        ));
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_millis(1000))
    .await
    .expect_or_log("application stopped unexpectedly");

    tracing::info!("application stopped");
}
