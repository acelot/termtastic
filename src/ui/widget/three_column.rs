use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};

#[derive(Clone)]
pub struct ThreeColumnWidget<T: Widget> {
    pub first: Option<T>,
    pub second: Option<T>,
    pub third: Option<T>,
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

        if let Some(first) = self.first {
            first.render(h[0], buf);
        }

        if let Some(second) = self.second {
            second.render(h[1], buf);
        }

        if let Some(third) = self.third {
            third.render(h[2], buf);
        }
    }
}
