use std::u16;

use tui_input::{Input as TuiInput, backend::crossterm::EventHandler};

use crate::ui::prelude::*;

pub struct ChatInput {
    placeholder: String,
    max_length: u16,
    input_component: TuiInput,
}

impl ChatInput {
    pub fn new(placeholder: String, max_length: u16) -> Self {
        Self {
            placeholder,
            max_length,
            input_component: TuiInput::default(),
        }
    }
}

impl Component for ChatInput {
    fn handle_event(&mut self, _state: &State, event: &Event, emit: &impl Fn(AppEvent)) {
        if let Event::Key(KeyEvent { code, .. }) = event {
            match code {
                KeyCode::Enter => emit(AppEvent::ChatMessageSubmitted(
                    self.input_component.value_and_reset(),
                )),
                KeyCode::Esc => self.input_component.reset(),
                _ => {}
            };
        }

        self.input_component.handle_event(event);
    }

    fn render(&mut self, _state: &State, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(7),
                Constraint::Min(1),
                Constraint::Length(8),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::from(" BHOP ").black().on_yellow(),
                Span::from(" ").on_blue(),
            ])),
            h[0],
        );

        let width = h[1].width.max(1) - 1;
        let scroll = self.input_component.visual_scroll(width as usize);

        let input = Paragraph::new(if !self.input_component.value().is_empty() {
            self.input_component.value()
        } else {
            &self.placeholder
        })
        .scroll((0, scroll as u16))
        .on_blue();

        frame.render_widget(input, h[1]);

        let x = self.input_component.visual_cursor().max(scroll) - scroll;
        frame.set_cursor_position((h[1].x + x as u16, h[1].y));

        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::from(format!(
                "{}/{}",
                self.input_component.value().len(),
                self.max_length
            ))]))
            .on_blue()
            .right_aligned(),
            h[2],
        );
    }
}
