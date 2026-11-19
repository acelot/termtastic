use crate::state::State;
use crate::types::UiEvent;
use crate::ui::screen::{Layout, Screen};
use crate::{APP_NAME, APP_VERSION};
use crossterm::terminal::SetTitle;
use crossterm::{
    event::{
        DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange, Event, EventStream,
        KeyCode, KeyEvent, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement,
    },
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
    ui_event_tx: broadcast::Sender<UiEvent>,
    state_rx: watch::Receiver<State>,
    crossterm_events: EventStream,
    layout: Layout<'a>,
}

impl<'a> Ui<'a> {
    pub fn new(ui_event_tx: broadcast::Sender<UiEvent>, state_rx: watch::Receiver<State>) -> Self {
        Self {
            ui_event_tx,
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
            Some(Ok(Event::Key(KeyEvent {
                code: KeyCode::F(12),
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }))) if modifiers.is_empty() => {
                terminal.clear()?;
                self.redraw(terminal)?;
            }
            Some(Ok(Event::Resize(_, _))) => {
                self.redraw(terminal)?;
            }
            Some(Ok(event)) => {
                if self.layout.handle_event(&self.state_rx.borrow(), &event, &|ev| {
                    let _ = self.ui_event_tx.send(ev);
                }) {
                    self.redraw(terminal)?;
                }
            }
            Some(Err(e)) => tracing::error!("event catching error {}", e),
            None => subsys.request_shutdown(),
        }

        Ok(())
    }

    fn redraw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
        let state = self.state_rx.borrow();

        set_terminal_title(state.terminal_title.as_ref())?;

        terminal.draw(|frame| {
            self.layout.render(&state, frame, frame.area(), &|ev| {
                let _ = self.ui_event_tx.send(ev);
            })
        })?;

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
    set_terminal_title(None)?;

    execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste, EnableFocusChange)?;

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

    execute!(
        stdout(),
        DisableFocusChange,
        DisableBracketedPaste,
        LeaveAlternateScreen
    )?;

    disable_raw_mode()?;

    Ok(())
}

fn set_terminal_title(title: Option<&String>) -> io::Result<()> {
    execute!(
        stdout(),
        SetTitle(format!(
            "{}{} {}",
            title.and_then(|t| Some(format!("{} · ", t))).unwrap_or_default(),
            APP_NAME,
            APP_VERSION
        ))
    )
}
