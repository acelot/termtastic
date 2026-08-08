use std::collections::HashMap;

use crate::service::TRACEROUTE_TIMEOUT_SECS;
use crate::types::{Hotkey, Node, TelemetryItem, Traceroute, TracerouteState};
use crate::ui::helpers::{
    Base64EncoderExt, ListStateExt, default_scrollbar, hops_to_spans, humanize_time_delta, humanize_uptime,
    last_heard_to_spans, routing_error_to_span, short_name_to_span,
};
use crate::ui::widget::{PlaceholderWidget, PopupConfirmWidget, TabsWidget, ThreeColumnWidget};
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    text::ToSpan,
    widgets::{Block, BorderType, Padding, Paragraph},
};
use strum::{Display, EnumCount, EnumIter, FromRepr, IntoEnumIterator};
use tui_widget_list::{ListBuilder, ListState, ListView};

#[derive(Debug, Default, Clone, Copy, PartialEq, FromRepr, Display, EnumIter, EnumCount)]
enum NodeInfoTab {
    #[default]
    #[strum(to_string = "info")]
    Info,
    #[strum(to_string = "traceroutes")]
    Traceroutes,
    #[strum(to_string = "position")]
    Position,
    #[strum(to_string = "telemetry")]
    Telemetry,
}

impl NodeInfoTab {
    pub fn prev(self) -> Self {
        let current_index: usize = self as usize;
        let (previous_index, overflowed) = current_index.overflowing_sub(1);

        Self::from_repr(if overflowed {
            NodeInfoTab::COUNT - 1
        } else {
            previous_index
        })
        .unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);

        Self::from_repr(if next_index > NodeInfoTab::COUNT - 1 {
            0
        } else {
            next_index
        })
        .unwrap_or(self)
    }
}

pub enum NodeInfoWidgetEvent {
    CloseRequested,
    CopyToClipboardRequested(String),
    NodeDeleteRequested,
    TracerouteRequested,
}

#[derive(Debug, Clone)]
pub struct NodeInfoContext<'a> {
    pub node_key: u32,
    pub nodes: &'a HashMap<u32, Node>,
    pub traceroutes: Vec<&'a Traceroute>,
    pub telemetry: &'a Vec<TelemetryItem>,
    pub uptime: Option<u32>,
    pub is_my_node: bool,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct NodeInfoWidgetState {
    active_tab: NodeInfoTab,
    traceroute_list_state: ListState,
    telemetry_list_state: ListState,
    is_delete_node_popup_visible: bool,
}


impl NodeInfoWidgetState {
    pub fn handle_event(
        &mut self,
        context: NodeInfoContext,
        event: Event,
        emit: &mut impl FnMut(NodeInfoWidgetEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        if self.is_delete_node_popup_visible {
            match event {
                Event::Key(KeyEvent {
                    code,
                    kind: KeyEventKind::Press,
                    modifiers,
                    ..
                }) if modifiers.is_empty() => match code {
                    KeyCode::Enter => {
                        emit(NodeInfoWidgetEvent::NodeDeleteRequested)?;
                        self.is_delete_node_popup_visible = false;
                    }
                    KeyCode::Esc => {
                        self.is_delete_node_popup_visible = false;
                    }
                    _ => {}
                },
                _ => {}
            }

            return Ok(true);
        }

        if self.active_tab == NodeInfoTab::Traceroutes
            && self.traceroute_list_state.handle_navigation_events(&event, None)
        {
            return Ok(true);
        }

        if self.active_tab == NodeInfoTab::Telemetry && self.telemetry_list_state.handle_navigation_events(&event, None)
        {
            return Ok(true);
        }

        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match (self.active_tab, code) {
                (_, KeyCode::Tab) if modifiers.is_empty() => {
                    self.active_tab = self.active_tab.next();
                    return Ok(true);
                }
                (_, KeyCode::BackTab) => {
                    self.active_tab = self.active_tab.prev();
                    return Ok(true);
                }
                (NodeInfoTab::Info, KeyCode::Char('k')) if modifiers.is_empty() => {
                    if let Some(user) = context.nodes.get(&context.node_key).and_then(|n| n.user.as_ref()) {
                        emit(NodeInfoWidgetEvent::CopyToClipboardRequested(
                            user.public_key.base64_encode(),
                        ))?;
                        return Ok(true);
                    }
                }
                (NodeInfoTab::Info, KeyCode::Delete | KeyCode::Backspace) if modifiers.is_empty() => {
                    self.is_delete_node_popup_visible = true;
                    return Ok(true);
                }
                (NodeInfoTab::Traceroutes, KeyCode::Char('r')) if modifiers.is_empty() => {
                    emit(NodeInfoWidgetEvent::TracerouteRequested)?;
                    return Ok(true);
                }
                (NodeInfoTab::Telemetry, KeyCode::Char('c')) if modifiers.is_empty() => {
                    if let Some(item) = self
                        .telemetry_list_state
                        .selected
                        .and_then(|i| context.telemetry.get(i))
                    {
                        match item {
                            TelemetryItem::Group { json, .. } => {
                                emit(NodeInfoWidgetEvent::CopyToClipboardRequested(json.to_owned()))?;
                            }
                            TelemetryItem::Item { value: Some(v), .. } => {
                                emit(NodeInfoWidgetEvent::CopyToClipboardRequested(v.to_owned()))?;
                            }
                            _ => {}
                        };

                        return Ok(true);
                    }
                }
                (_, KeyCode::Esc) if modifiers.is_empty() => {
                    emit(NodeInfoWidgetEvent::CloseRequested)?;
                    return Ok(true);
                }
                _ => {}
            },
            _ => {}
        }

        Ok(false)
    }

    pub fn get_hotkeys(&self, is_my_node: bool) -> Vec<Hotkey> {
        match &self.active_tab {
            NodeInfoTab::Info => vec![
                Some(Hotkey::new("esc", "close")),
                Some(Hotkey::new("k", "copy public key")),
                (!is_my_node).then_some(Hotkey::new("delete", "remove")),
            ]
            .into_iter()
            .flatten()
            .collect(),
            NodeInfoTab::Traceroutes => vec![Hotkey::new("esc", "close"), Hotkey::new("r", "run traceroute")],
            NodeInfoTab::Telemetry => vec![Hotkey::new("esc", "close"), Hotkey::new("c", "copy")],
            _ => vec![],
        }
    }
}

pub struct NodeInfoWidget<'a> {
    context: NodeInfoContext<'a>,
}

impl<'a> NodeInfoWidget<'a> {
    pub fn new(context: NodeInfoContext<'a>) -> Self {
        Self { context }
    }

    fn render_info(&self, node: &Node, area: Rect, buf: &mut Buffer, state: &mut NodeInfoWidgetState) {
        let v = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(area);

        // first line
        ThreeColumnWidget {
            first: Some(InfoWidget::new("short name", node.short_name().to_span())),
            second: Some(InfoWidget::new("node number", node.key.to_span())),
            third: Some(InfoWidget::new("user ID", node.id().to_span())),
        }
        .render(v[0], buf);

        // second line
        ThreeColumnWidget {
            first: Some(InfoWidget::new(
                "last heard",
                last_heard_to_spans(node, self.context.is_my_node),
            )),
            second: Some(InfoWidget::new("hops", hops_to_spans(node, self.context.is_my_node))),
            third: Some(InfoWidget::new(
                "uptime",
                self.context
                    .uptime.map(|s| Span::from(humanize_uptime(s)))
                    .unwrap_or(Span::from("no data").dark_gray()),
            )),
        }
        .render(v[1], buf);

        // third line
        ThreeColumnWidget {
            first: Some(InfoWidget::new("device role", node.role().to_span())),
            second: Some(InfoWidget::new(
                "public key",
                if let Some(user) = node.user.as_ref() {
                    Span::from(format!("{}-byte", user.public_key.len())).green()
                } else {
                    "none".to_span().red()
                },
            )),
            third: Some(InfoWidget::new(
                "status",
                if node.user.is_none() {
                    Span::from("UNKNOWN").yellow()
                } else {
                    Span::from("STORED").green()
                },
            )),
        }
        .render(v[2], buf);

        // fourth line
        InfoWidget::new("hardware", node.hw_model().to_span().magenta()).render(v[3], buf);

        // delete popup
        if state.is_delete_node_popup_visible {
            PopupConfirmWidget::new(
                "This node will be removed from your list until your node receives data from it again.",
                "confirm",
                "cancel",
                40,
                Color::Red,
            )
            .render(area, buf);
        }
    }

    fn render_traceroutes(
        &self,
        traceroutes: &Vec<&Traceroute>,
        area: Rect,
        buf: &mut Buffer,
        state: &mut NodeInfoWidgetState,
    ) {
        if self.context.is_my_node {
            PlaceholderWidget::dark_gray("not available for node").render(area, buf);
            return;
        }

        let v = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(area);

        // list
        let v0_h = Layout::horizontal([
            Constraint::Fill(4),
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Fill(2),
        ])
        .split(v[0]);

        Line::from(vec![Span::from("STATE").magenta()]).render(v0_h[0], buf);
        Line::from(vec![Span::from("ACK").magenta()]).render(v0_h[1], buf);
        Line::from(vec![Span::from("TOWARDS").magenta()]).render(v0_h[2], buf);
        Line::from(vec![Span::from("BACK").magenta()]).render(v0_h[3], buf);
        Line::from(vec![Span::from("TIME").magenta()]).render(v0_h[4], buf);
        Line::from(vec![Span::from("WHEN").magenta()])
            .right_aligned()
            .render(v0_h[5], buf);

        if traceroutes.is_empty() {
            PlaceholderWidget::dark_gray("press <r> to run traceroute").render(v[1], buf);
            return;
        };

        state.traceroute_list_state.fix_selection(traceroutes.len());

        let list_builder = ListBuilder::new(|context| {
            let widget = TracerouteWidget {
                item: traceroutes[context.index],
                is_selected: context.is_selected,
            };

            (widget, 1)
        });

        let list = ListView::new(list_builder, traceroutes.len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        list.render(v[1], buf, &mut state.traceroute_list_state);
    }

    fn render_telemetry(
        &self,
        telemetry: &Vec<TelemetryItem>,
        area: Rect,
        buf: &mut Buffer,
        state: &mut NodeInfoWidgetState,
    ) {
        if telemetry.is_empty() {
            PlaceholderWidget::dark_gray("no telemetry collected yet").render(area, buf);
            return;
        };

        state.telemetry_list_state.fix_selection(telemetry.len());

        let list_builder = ListBuilder::new(|context| {
            let widget = TelemetryItemWidget {
                item: &telemetry[context.index],
                is_selected: context.is_selected,
            };

            (widget, 1)
        });

        let list = ListView::new(list_builder, telemetry.len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        list.render(area, buf, &mut state.telemetry_list_state);
    }
}

impl<'a> StatefulWidget for NodeInfoWidget<'a> {
    type State = NodeInfoWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let maybe_node = self.context.nodes.get(&self.context.node_key);

        let title = match maybe_node {
            Some(node) => Line::from(vec![
                Span::from(" "),
                short_name_to_span(node, self.context.is_my_node),
                Span::from(" "),
                Span::from(node.long_name()).bold(),
                Span::from(" "),
            ]),
            None => Line::from(Span::from("Node not found").dark_gray()),
        };

        let block = Block::bordered()
            .border_type(BorderType::Thick)
            .padding(Padding::symmetric(2, 1))
            .title(title);

        let block_area = block.inner(area);
        block.render(area, buf);

        let Some(node) = maybe_node else {
            PlaceholderWidget::dark_gray("node not found").render(block_area, buf);
            return;
        };

        let v = Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Fill(1)]).split(block_area);

        // tabs
        TabsWidget::new(
            NodeInfoTab::iter().map(|t| (t as usize, t.to_string())).collect(),
            state.active_tab as usize,
        )
        .render(v[0], buf);

        match &state.active_tab {
            NodeInfoTab::Info => self.render_info(node, v[2], buf, state),
            NodeInfoTab::Traceroutes => self.render_traceroutes(&self.context.traceroutes, v[2], buf, state),
            NodeInfoTab::Telemetry => self.render_telemetry(self.context.telemetry, v[2], buf, state),
            _ => PlaceholderWidget::red("not implemented").render(v[2], buf),
        }
    }
}

#[derive(Clone)]
struct InfoWidget<'a> {
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

struct TracerouteWidget<'a> {
    item: &'a Traceroute,
    is_selected: bool,
}

impl<'a> Widget for TracerouteWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::new();
        let block_area = block.inner(area);

        block.render(area, buf);

        let h = Layout::horizontal([
            Constraint::Fill(4),
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Fill(3),
            Constraint::Fill(2),
            Constraint::Fill(2),
        ])
        .split(block_area);

        let selected_modifier = if self.is_selected {
            Modifier::UNDERLINED | Modifier::BOLD
        } else {
            Modifier::empty()
        };

        match &self.item.state {
            TracerouteState::Started => {
                Span::from("pending")
                    .add_modifier(selected_modifier)
                    .yellow()
                    .render(h[0], buf);

                Span::from(humanize_uptime(
                    Utc::now().signed_duration_since(self.item.datetime).num_seconds() as u32,
                ))
                .dark_gray()
                .render(h[4], buf);
            }
            TracerouteState::RoutingError => {
                routing_error_to_span(self.item.routing_error)
                    .add_modifier(selected_modifier)
                    .render(h[0], buf);

                Span::from(humanize_uptime(self.item.duration.num_seconds() as u32))
                    .dark_gray()
                    .render(h[4], buf);
            }
            TracerouteState::TimedOut => {
                Span::from("timed out")
                    .dark_gray()
                    .add_modifier(selected_modifier)
                    .render(h[0], buf);

                Span::from(humanize_uptime(TRACEROUTE_TIMEOUT_SECS as u32))
                    .dark_gray()
                    .render(h[4], buf);
            }
            TracerouteState::Finished => {
                Span::from("finished")
                    .green()
                    .add_modifier(selected_modifier)
                    .render(h[0], buf);

                Span::from(humanize_uptime(self.item.duration.num_seconds() as u32))
                    .dark_gray()
                    .render(h[4], buf);
            }
        }

        if self.item.acked {
            Span::from("\u{2714}").green().render(h[1], buf);
        } else {
            Span::from("–").dark_gray().render(h[1], buf);
        }

        if !self.item.route_towards.is_empty() {
            Line::from(hops_to_spans(&self.item.route_towards, false)).render(h[2], buf);
        } else {
            Span::from("–").dark_gray().render(h[2], buf);
        }

        if !self.item.route_back.is_empty() {
            Line::from(hops_to_spans(&self.item.route_back, false)).render(h[3], buf);
        } else {
            Span::from("–").dark_gray().render(h[3], buf);
        }

        Line::from(humanize_time_delta(
            Utc::now().signed_duration_since(self.item.datetime),
        ))
        .right_aligned()
        .render(h[5], buf);
    }
}

struct TelemetryItemWidget<'a> {
    item: &'a TelemetryItem,
    is_selected: bool,
}

impl<'a> Widget for TelemetryItemWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.item {
            TelemetryItem::Group { title, datetime, .. } => {
                let h =
                    Layout::horizontal([Constraint::Fill(3), Constraint::Fill(2), Constraint::Length(2)]).split(area);

                Line::from(vec![Span::from(title).bold().add_modifier(if self.is_selected {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                })])
                .magenta()
                .render(h[0], buf);

                Line::from(humanize_time_delta(Utc::now().signed_duration_since(datetime)))
                    .right_aligned()
                    .render(h[1], buf);
            }
            TelemetryItem::Item {
                title, formatted_value, ..
            } => {
                let v = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

                Line::from(vec![
                    Span::from("  "),
                    Span::from(format!("{}:", title)).add_modifier(if self.is_selected {
                        Modifier::UNDERLINED | Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                ])
                .render(v[0], buf);

                formatted_value
                    .as_ref().map(Span::from)
                    .unwrap_or(Span::from("no data").dark_gray())
                    .add_modifier(if self.is_selected {
                        Modifier::UNDERLINED | Modifier::BOLD
                    } else {
                        Modifier::empty()
                    })
                    .render(v[1], buf);
            }
        }
    }
}
