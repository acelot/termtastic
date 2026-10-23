use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

#[derive(Clone)]
pub struct InfoWidget<'a> {
    pub title: &'a str,
    pub value: Line<'a>,
}

impl<'a> InfoWidget<'a> {
    pub fn new(title: &'a str, value: impl Into<Line<'a>>) -> Self {
        Self {
            title,
            value: value.into(),
        }
    }
}

impl<'a> Widget for InfoWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        Paragraph::new(vec![Line::from(Span::from(self.title).dark_gray()), self.value]).render(area, buf);
    }
}
