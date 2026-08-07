use crate::state::State;
use crate::types::AppEvent;
use crate::ui::component::{Component, Layout};
use crossterm::event::{
    DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::terminal::supports_keyboard_enhancement;
use crossterm::{
    event::{Event, EventStream},
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

pub struct Ui<'a> {
    app_event_tx: broadcast::Sender<AppEvent>,
    state_rx: watch::Receiver<State>,
    crossterm_events: EventStream,
    layout: Layout<'a>,
}

impl<'a> Ui<'a> {
    pub fn new(app_event_tx: broadcast::Sender<AppEvent>, state_rx: watch::Receiver<State>) -> Self {
        Self {
            app_event_tx,
            state_rx,
            crossterm_events: EventStream::new(),
            layout: Layout::new(),
        }
    }

    pub async fn run(&mut self, subsys: &mut SubsystemHandle) -> anyhow::Result<()> {
        let supports_keyboard_enhancement = supports_keyboard_enhancement().unwrap_or(false);
        let mut terminal = setup_terminal(supports_keyboard_enhancement)?;

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

        restore_terminal(supports_keyboard_enhancement)?;

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
                self.layout.handle_event(&self.state_rx.borrow(), &event, &|ev| {
                    self.app_event_tx.send(ev)?;
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
        terminal.draw(|frame| self.layout.render(&self.state_rx.borrow(), frame, frame.area()))?;

        Ok(())
    }
}

fn setup_terminal(supports_keyboard_enhancement: bool) -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    let original_hook = take_hook();

    set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal(supports_keyboard_enhancement);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;

    execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste)?;

    if supports_keyboard_enhancement {
        execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }

    Terminal::new(CrosstermBackend::new(stdout()))
}

fn restore_terminal(supports_keyboard_enhancement: bool) -> io::Result<()> {
    if supports_keyboard_enhancement {
        execute!(stdout(), PopKeyboardEnhancementFlags)?;
    }

    execute!(stdout(), DisableBracketedPaste, LeaveAlternateScreen)?;

    disable_raw_mode()?;

    Ok(())
}
