use ratatui::{
    prelude::*,
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
            .map(|Hotkey { key, alt, label }| {
                [
                    Some(Span::from(key.clone())),
                    alt.is_some().then_some(Span::from("/").dark_gray()),
                    alt.as_ref().and_then(|a| Some(Span::from(a))),
                    Some(Span::from("\u{00A0}")),
                    Some(Span::from(label.clone()).dark_gray()),
                    Some(Span::from("  ")),
                ]
                .into_iter()
                .flatten()
            })
            .flatten()
            .collect();

        Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
