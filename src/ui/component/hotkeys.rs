use crate::ui::prelude::*;

pub struct Hotkeys {
    items: Vec<Hotkey>,
}

impl Hotkeys {
    pub fn new(items: Vec<Hotkey>) -> Self {
        Self { items }
    }
}

impl Component for Hotkeys {
    fn handle_event(
        &mut self,
        _state: &State,
        _event: &Event,
        _emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn render(&mut self, _state: &State, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = self.items.iter().flat_map(|h| Vec::from(h)).collect();
        let paragraph = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }
}

impl From<&Hotkey> for Vec<Span<'static>> {
    fn from(value: &Hotkey) -> Self {
        vec![
            Span::from(value.key.clone()),
            Span::from("\u{00A0}"),
            Span::from(value.label.clone()).dark_gray(),
            Span::from("  "),
        ]
    }
}
