use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::Widget,
};

pub struct PlaceholderWidget<'a> {
    text: &'a str,
    color: Color,
}

impl<'a> PlaceholderWidget<'a> {
    pub fn dark_gray(text: &'a str) -> Self {
        Self {
            text,
            color: Color::DarkGray,
        }
    }

    pub fn red(text: &'a str) -> Self {
        Self {
            text,
            color: Color::Red,
        }
    }
}

impl<'a> Widget for PlaceholderWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ])
            .split(area);

        Line::from(Span::from(self.text))
            .fg(self.color)
            .centered()
            .render(v[1], buf);
    }
}
