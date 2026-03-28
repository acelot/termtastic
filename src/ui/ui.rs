use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::{StreamExt, future::FutureExt};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::{
    io::{self, Stdout, stdout},
    panic::{set_hook, take_hook},
};
use tokio::sync::{broadcast, watch};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing_unwrap::ResultExt;

use crate::ui::{component::Component, component::Layout};
use crate::{state::State, types::AppEvent};

pub struct Ui {
    state_rx: watch::Receiver<State>,
    event_tx: broadcast::Sender<AppEvent>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    crossterm_events: EventStream,
    layout: Layout,
}

impl Ui {
    pub fn new(state_rx: watch::Receiver<State>, event_tx: broadcast::Sender<AppEvent>) -> Self {
        Self {
            state_rx,
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
                if let Event::Key(KeyEvent {
                    code, modifiers, ..
                }) = event
                    && code == KeyCode::Char('c')
                    && modifiers.contains(KeyModifiers::CONTROL)
                {
                    subsys.request_shutdown();
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
        self.terminal
            .draw(|frame| {
                self.layout
                    .render(&self.state_rx.borrow(), frame, frame.area())
            })
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
