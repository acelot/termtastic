use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};

#[derive(Clone)]
pub struct ThreeColumnWidget<T: Widget> {
    pub col1: Option<T>,
    pub col2: Option<T>,
    pub col3: Option<T>,
}

impl<T: Widget> Widget for ThreeColumnWidget<T> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let h = Layout::horizontal([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(area);

        if let Some(first) = self.col1 {
            first.render(h[0], buf);
        }

        if let Some(second) = self.col2 {
            second.render(h[1], buf);
        }

        if let Some(third) = self.col3 {
            third.render(h[2], buf);
        }
    }
}
