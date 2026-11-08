use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::{state::State, ui::widget::FontWidget};

pub struct OnlineNodesWidget {
    data: OnlineNodesWidgetData,
}

impl OnlineNodesWidget {
    pub fn new(data: OnlineNodesWidgetData) -> Self {
        Self { data }
    }
}

impl Widget for OnlineNodesWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Plain)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(" online nodes ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let online = self.data.online.to_string();

        let num_widget = if block_area.height > 8 {
            FontWidget::large_bold(&online, 1)
        } else {
            FontWidget::medium_bold(&online, 0)
        };

        let (num_widget_width, num_widget_height) = num_widget.size();

        let v = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(num_widget_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(block_area);

        let v1_h = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(num_widget_width),
            Constraint::Min(0),
        ])
        .split(v[1]);

        num_widget.render(v1_h[1], buf);

        Line::from(vec![
            Span::from("total: ").dark_gray(),
            Span::from(self.data.total.to_string()),
        ])
        .right_aligned()
        .render(v[3], buf);
    }
}

pub struct OnlineNodesWidgetData {
    online: usize,
    total: usize,
}

impl From<&State> for OnlineNodesWidgetData {
    fn from(value: &State) -> Self {
        Self {
            online: value.online_nodes as usize,
            total: value.nodes.len(),
        }
    }
}
