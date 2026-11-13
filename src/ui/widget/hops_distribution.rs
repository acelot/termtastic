use std::{collections::HashMap, ops::Deref};

use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Padding},
};

use crate::state::State;

pub struct HopsDistributionWidget {
    data: HopsDistributionWidgetData,
}

impl HopsDistributionWidget {
    pub fn new(data: HopsDistributionWidgetData) -> Self {
        Self { data }
    }
}

impl Widget for HopsDistributionWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new()
            .borders(Borders::all())
            .border_type(BorderType::Rounded)
            .border_style(Style::new().dark_gray())
            .padding(Padding::symmetric(1, 0))
            .title_top(" nodes per hop ");

        let block_area = block.inner(area);
        block.render(area, buf);

        let v = Layout::vertical(Constraint::from_lengths([1, 1, 1, 1, 1, 1, 1, 1])).split(block_area);

        let (hop_max_online, hop_max_total) =
            self.data
                .iter()
                .fold((0u32, 0u32), |(mut mo, mut mt), (_, (online, total))| {
                    if *online > mo {
                        mo = *online;
                    }

                    if *total > mt {
                        mt = *total;
                    }

                    (mo, mt)
                });

        let online_digits = hop_max_online.checked_ilog10().unwrap_or(0) as u16 + 1;
        let total_digits = hop_max_total.checked_ilog10().unwrap_or(0) as u16 + 1;

        for hop in 0..=7u32 {
            let h = Layout::horizontal([
                Constraint::Length(3),
                Constraint::Fill(1),
                Constraint::Length(2),
                Constraint::Length(online_digits),
                Constraint::Length(1),
                Constraint::Length(total_digits),
            ])
            .split(v[hop as usize]);

            Span::from(if hop > 0 { hop.to_string() } else { "*".to_owned() }).render(h[0], buf);

            let (hop_online, hop_total) = self.data[&hop];

            let online_percent = hop_online as f32 * 100.0 / hop_max_total as f32;
            let total_percent = hop_total as f32 * 100.0 / hop_max_total as f32;
            let online_width: usize = (h[1].width as f32 * online_percent / 100.0).round() as usize;
            let total_width: usize = (h[1].width as f32 * total_percent / 100.0).round() as usize;

            Line::from(vec![
                Span::from("■".repeat(online_width)).green(),
                Span::from("■".repeat(total_width.saturating_sub(online_width)))
                    .dark_gray()
                    .dim(),
            ])
            .render(h[1], buf);

            Line::from(Span::from(hop_online.to_string()).green())
                .right_aligned()
                .render(h[3], buf);

            Line::from(Span::from(hop_total.to_string()).dark_gray())
                .right_aligned()
                .render(h[5], buf);
        }
    }
}

pub struct HopsDistributionWidgetData(HashMap<u32, (u32, u32)>);

impl Deref for HopsDistributionWidgetData {
    type Target = HashMap<u32, (u32, u32)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&State> for HopsDistributionWidgetData {
    fn from(value: &State) -> Self {
        Self(value.nodes.iter().fold(
            HashMap::from([
                (0, (0, 0)),
                (1, (0, 0)),
                (2, (0, 0)),
                (3, (0, 0)),
                (4, (0, 0)),
                (5, (0, 0)),
                (6, (0, 0)),
                (7, (0, 0)),
            ]),
            |mut acc, (_, node)| {
                if let Some(hops) = node.hops {
                    acc.entry(hops).and_modify(|(online, total)| {
                        if node.is_online() {
                            *online += 1;
                        }

                        *total += 1;
                    });
                }

                acc
            },
        ))
    }
}
