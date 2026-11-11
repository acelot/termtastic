use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};

use crate::ui::prelude::*;

pub struct Dashboard {
    scroll_view_state: ScrollViewState,
    hardware_distribution_state: HardwareDistributionWidgetState,
    role_distribution_state: RoleDistributionWidgetState,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            scroll_view_state: ScrollViewState::default(),
            hardware_distribution_state: HardwareDistributionWidgetState::default(),
            role_distribution_state: RoleDistributionWidgetState::default(),
        }
    }
}

impl Screen for Dashboard {
    fn get_hotkeys(&self, _state: &State) -> Vec<Hotkey> {
        vec![Hotkey::new("↑↓".to_string(), "scroll".to_string())]
    }

    fn handle_event(&mut self, _state: &State, event: &Event, _emit: &impl Fn(UiEvent)) -> bool {
        match event {
            Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                modifiers,
                ..
            }) => match code {
                KeyCode::Up if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_up();
                    return true;
                }
                KeyCode::Down if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_down();
                    return true;
                }
                _ => {}
            },
            Event::Mouse(MouseEvent { kind, modifiers, .. }) => match kind {
                MouseEventKind::ScrollUp if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_up();
                    return true;
                }
                MouseEventKind::ScrollDown if modifiers.is_empty() => {
                    self.scroll_view_state.scroll_down();
                    return true;
                }
                _ => {}
            },
            _ => {}
        }

        false
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let is_big = area.width > 116;
        let rows: [u16; 4] = if is_big { [8, 10, 14, 9] } else { [8, 10, 12, 9] };

        let content_size = Size::new(area.width - 2, rows.iter().sum());
        let mut scroll_view = ScrollView::new(content_size)
            .horizontal_scrollbar_visibility(ScrollbarVisibility::Never)
            .vertical_scrollbar_symbols(DEFAULT_SCROLLBAR_SET)
            .vertical_scrollbar_style(Style::new().dark_gray());

        let v = Layout::vertical([
            Constraint::Length(rows[0]),
            Constraint::Length(rows[1]),
            Constraint::Length(rows[2]),
            Constraint::Length(rows[3]),
        ])
        .split(Rect::new(0, 0, content_size.width, content_size.height));

        // first row
        let v0_h = Layout::horizontal(Constraint::from_fills([1, 1, 1, if is_big { 1 } else { 0 }])).split(v[0]);

        scroll_view.render_widget(MyNodeWidget::new(state.into()), v0_h[0]);
        scroll_view.render_widget(UptimeWidget::new(state.into()), v0_h[1]);
        scroll_view.render_widget(OnlineNodesWidget::new(state.into()), v0_h[2]);
        scroll_view.render_widget(ClockWidget, v0_h[3]);

        // second row
        let v1_h = Layout::horizontal(Constraint::from_fills([1, 1, 1])).split(v[1]);

        scroll_view.render_widget(HopsDistributionWidget::new(state.into()), v1_h[0]);
        scroll_view.render_stateful_widget(
            RoleDistributionWidget::new(state.into()),
            v1_h[1],
            &mut self.role_distribution_state,
        );
        scroll_view.render_stateful_widget(
            HardwareDistributionWidget::new(state.into()),
            v1_h[2],
            &mut self.hardware_distribution_state,
        );

        // third row
        scroll_view.render_widget(DeviceMetricsGraphWidget::new(state.into()), v[2]);

        // fourth row
        let v3_h = Layout::horizontal(Constraint::from_fills([1, 1, 1])).split(v[3]);

        let node_distance_widget_data: NodeDistanceWidgetData = state.into();

        scroll_view.render_widget(
            NodeDistanceWidget::new(NodeDistanceWidgetMode::Closest, &node_distance_widget_data),
            v3_h[0],
        );

        scroll_view.render_widget(
            NodeDistanceWidget::new(NodeDistanceWidgetMode::Farthest, &node_distance_widget_data),
            v3_h[1],
        );

        // render scroll view
        scroll_view.render(area, frame.buffer_mut(), &mut self.scroll_view_state);
    }
}
