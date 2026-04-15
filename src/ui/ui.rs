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

use crate::{state::State, types::AppEvent};
use crate::{
    state::StateAction,
    ui::component::{Component, Layout},
};

pub struct Ui<'a> {
    state_rx: watch::Receiver<State>,
    state_action_tx: mpsc::UnboundedSender<StateAction>,
    event_tx: broadcast::Sender<AppEvent>,
    crossterm_events: EventStream,
    layout: Layout<'a>,
}

impl<'a> Ui<'a> {
    pub fn new(
        state_rx: watch::Receiver<State>,
        state_action_tx: mpsc::UnboundedSender<StateAction>,
        event_tx: broadcast::Sender<AppEvent>,
    ) -> Self {
        Self {
            state_rx,
            state_action_tx,
            event_tx,
            crossterm_events: EventStream::new(),
            layout: Layout::new(),
        }
    }

    pub async fn run(&mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let mut terminal = setup_terminal()?;

        self.redraw(&mut terminal)?;

        loop {
            tokio::select! {
                maybe_event = self.crossterm_events.next().fuse() => self.handle_crossterm_event(
                    maybe_event,
                    &mut terminal,
                    subsys
                )?,
                _ = self.state_rx.changed() => self.redraw(&mut terminal)?,
                _ = subsys.on_shutdown_requested() => {
                    tracing::info!("shutdown");
                    break;
                },
            }
        }

        restore_terminal()?;

        Ok(())
    }

    fn handle_crossterm_event(
        &mut self,
        maybe_event: Option<Result<Event, io::Error>>,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        subsys: &mut SubsystemHandle,
    ) -> anyhow::Result<()> {
        match maybe_event {
            Some(Ok(event)) => {
                if let Event::Key(key_event) = event
                    && key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    subsys.request_shutdown();
                }

                self.layout
                    .handle_event(&self.state_rx.borrow(), &event, &|ev| {
                        self.event_tx.send(ev)?;
                        Ok(())
                    })?;

                self.redraw(terminal)?;
            }
            Some(Err(e)) => tracing::error!("event catching error {}", e),
            None => subsys.request_shutdown(),
        }

        Ok(())
    }

    fn redraw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        let state = &self.state_rx.borrow();

        if state.need_clear_frame {
            terminal.clear()?;
            self.state_action_tx.send(StateAction::FrameCleared)?;

            return Ok(());
        }

        terminal.draw(|frame| self.layout.render(state, frame, frame.area()))?;

        Ok(())
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
