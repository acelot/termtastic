use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::{StreamExt, future::FutureExt};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::{
    io::{self, Stdout, stdout},
    panic::{set_hook, take_hook},
};
use tokio::sync::{broadcast, mpsc, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::{state::State, types::AppEvent};
use crate::{
    state::StateAction,
    ui::component::{Component, Layout},
};

pub struct Ui {
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    event_tx: broadcast::Sender<AppEvent>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    crossterm_events: EventStream,
    layout: Layout,
}

impl Ui {
    pub fn new(
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        event_tx: broadcast::Sender<AppEvent>,
    ) -> Self {
        Self {
            state_rx,
            state_action_tx,
            event_tx,
            terminal: setup_terminal().unwrap_or_log(),
            crossterm_events: EventStream::new(),
            layout: Layout::new(),
        }
    }

    pub async fn run(&mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        self.redraw();

        loop {
            tokio::select! {
                maybe_event = self.crossterm_events.next().fuse() => self.handle_crossterm_event(
                    maybe_event,
                    subsys
                ),
                _ = self.state_rx.changed() => self.redraw(),
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                },
            }
        }

        restore_terminal().unwrap_or_log();

        Ok(())
    }

    fn handle_crossterm_event(
        &mut self,
        maybe_event: Option<Result<Event, io::Error>>,
        subsys: &mut SubsystemHandle,
    ) {
        match maybe_event {
            Some(Ok(event)) => {
                if let Event::Key(key_event) = event
                    && key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    subsys.request_shutdown();
                }

                if let Event::Resize(_, _) = event {
                    self.terminal.clear().unwrap_or_log();
                }

                self.layout
                    .handle_event(&self.state_rx.borrow(), &event, &|ev| {
                        self.event_tx.send(ev).unwrap_or_log();
                    });

                self.redraw();
            }
            Some(Err(e)) => tracing::error!("event catching error {}", e),
            None => subsys.request_shutdown(),
        }
    }

    fn redraw(&mut self) {
        let state = &self.state_rx.borrow();

        if state.need_clear_frame {
            self.terminal.clear().unwrap_or_log();
            self.state_action_tx
                .send(StateAction::FrameCleared)
                .unwrap_or_log();

            return;
        }

        self.terminal
            .draw(|frame| self.layout.render(state, frame, frame.area()))
            .unwrap_or_log();
    }
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    let original_hook = take_hook();

    set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    Terminal::new(CrosstermBackend::new(stdout()))
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;

    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}
