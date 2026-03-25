use std::fmt::Write;

use chrono::Utc;
use tokio::sync::mpsc;
use tracing::field::{Field, Visit};
use tracing_subscriber::Layer;

use crate::{state::StateAction, types::LogRecord};

pub struct LogToState {
    state_action_tx: mpsc::UnboundedSender<StateAction>,
}

impl LogToState {
    pub fn new(state_action_tx: mpsc::UnboundedSender<StateAction>) -> Self {
        Self { state_action_tx }
    }
}

impl<S: tracing::Subscriber + Send + Sync> Layer<S> for LogToState {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = *event.metadata().level();
        let source = event.metadata().target().to_string();

        let mut message = String::new();
        let mut visitor = MessageVisitor {
            message: &mut message,
        };

        event.record(&mut visitor);

        let _ = self
            .state_action_tx
            .send(StateAction::LogRecordAdd(LogRecord {
                datetime: Utc::now(),
                level,
                source,
                message,
            }));
    }
}

struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> Visit for MessageVisitor<'a> {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            write!(self.message, "{:?}", value).expect("Failed to write message");
        }
    }
}
