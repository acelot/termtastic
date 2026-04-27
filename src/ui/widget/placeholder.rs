use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Span,
    widgets::{Paragraph, Widget, Wrap},
};

pub struct PlaceholderWidget<'a> {
    text: Paragraph<'a>,
}

impl<'a> PlaceholderWidget<'a> {
    pub fn new(text: Paragraph<'a>) -> Self {
        Self { text }
    }

    pub fn dark_gray(text: &'a str) -> Self {
        Self {
            text: Paragraph::new(Span::from(text).dark_gray())
                .centered()
                .wrap(Wrap { trim: false }),
        }
    }

    pub fn black_on_dark_gray(text: &'a str) -> Self {
        Self {
            text: Paragraph::new(Span::from(text).black().on_dark_gray())
                .centered()
                .wrap(Wrap { trim: false }),
        }
    }

    pub fn red(text: &'a str) -> Self {
        Self {
            text: Paragraph::new(Span::from(text).red())
                .centered()
                .wrap(Wrap { trim: false }),
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
                Constraint::Length(self.text.line_count(area.width) as u16),
                Constraint::Fill(1),
            ])
            .split(area);

        self.text.render(v[1], buf);
    }
}
