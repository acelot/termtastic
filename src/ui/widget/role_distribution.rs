use ordermap::OrderMap;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding},
};
use tui_widget_list::{ListBuilder, ListState, ListView};

use crate::state::State;

pub struct RoleDistributionWidget {
    data: RoleDistributionWidgetData,
}

#[derive(Default)]
pub struct RoleDistributionWidgetState {
    list_state: ListState,
}

impl RoleDistributionWidget {
    pub fn new(data: RoleDistributionWidgetData) -> Self {
        Self { data }
    }
}

impl StatefulWidget for RoleDistributionWidget {
    type State = RoleDistributionWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(" top roles ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let online_length = self.data.max_online.checked_ilog10().unwrap_or(0) as u16 + 1;
        let total_length = self.data.max_total.checked_ilog10().unwrap_or(0) as u16 + 1;

        let list_builder = ListBuilder::new(|context| {
            let (role, (online, total)) = self.data.stat.get_index(context.index).unwrap();

            let item = ListItemWidget {
                index: context.index,
                role,
                online: *online,
                total: *total,
                online_length,
                total_length,
            };

            (item, 1)
        });

        let list = ListView::new(list_builder, self.data.stat.len());

        list.render(block_area, buf, &mut state.list_state);
    }
}

struct ListItemWidget<'a> {
    pub index: usize,
    pub role: &'a String,
    pub online: u32,
    pub total: u32,
    pub online_length: u16,
    pub total_length: u16,
}

impl<'a> Widget for ListItemWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let h = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Length(self.online_length),
            Constraint::Length(1),
            Constraint::Length(self.total_length),
        ])
        .split(area);

        Span::from((self.index + 1).to_string())
            .dark_gray()
            .dim()
            .render(h[0], buf);

        Span::from(self.role).render(h[2], buf);

        Line::from(Span::from(self.online.to_string()).green())
            .right_aligned()
            .render(h[4], buf);

        Line::from(Span::from(self.total.to_string()).dark_gray())
            .right_aligned()
            .render(h[6], buf);
    }
}

pub struct RoleDistributionWidgetData {
    stat: OrderMap<String, (u32, u32)>,
    max_online: u32,
    max_total: u32,
}

impl From<&State> for RoleDistributionWidgetData {
    fn from(value: &State) -> Self {
        let mut stat: OrderMap<String, (u32, u32)> = OrderMap::new();

        for node in value.nodes.values() {
            let (online, total) = stat.entry(node.role()).or_default();

            *total += 1;

            if node.is_online() {
                *online += 1;
            }
        }

        stat.sort_by(|_, (_, t1), _, (_, t2)| t1.cmp(t2).reverse());

        let max_online = stat.values().map(|(online, _)| *online).max().unwrap_or(0);
        let max_total = stat.values().map(|(_, total)| *total).max().unwrap_or(0);

        Self {
            stat,
            max_online,
            max_total,
        }
    }
}
