use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::types::Hotkey;

pub struct HotkeysWidget<'a> {
    items: &'a Vec<Hotkey>,
}

impl<'a> HotkeysWidget<'a> {
    pub fn new(items: &'a Vec<Hotkey>) -> Self {
        Self { items }
    }
}

impl<'a> Widget for HotkeysWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let spans: Vec<Span> = self
            .items
            .iter()
            .flat_map(|hotkey| {
                vec![
                    Span::from(hotkey.key.clone()),
                    Span::from("\u{00A0}"),
                    Span::from(hotkey.label.clone()).dark_gray(),
                    Span::from("  "),
                ]
            })
            .collect();

        Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
