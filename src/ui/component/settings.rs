use crate::{
    service::{FORMS, SETTINGS},
    ui::{helpers::default_scrollbar, prelude::*},
};

pub struct Settings {
    settings_list_state: ListState,
    form_list_state: ListState,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            settings_list_state: ListState::default(),
            form_list_state: ListState::default(),
        }
    }

    fn get_hotkeys(&self, form_state: &SettingsFormState) -> Vec<Hotkey> {
        match form_state {
            SettingsFormState::Inactive => vec![
                Some(Hotkey::new("↑↓", "scroll")),
                Some(Hotkey::new("enter", "open")),
            ],
            SettingsFormState::Loading { .. } => vec![Some(Hotkey::new("esc", "cancel"))],
            SettingsFormState::LoadingFailed { .. } => vec![Some(Hotkey::new("esc", "return"))],
            SettingsFormState::Loaded { .. } => vec![
                Some(Hotkey::new("↑↓", "scroll")),
                self.form_list_state
                    .selected
                    .is_some()
                    .then_some(Hotkey::new("enter", "edit")),
                Some(Hotkey::new("esc", "return")),
            ],
            SettingsFormState::Saving { .. } => vec![Some(Hotkey::new("esc", "return"))],
            SettingsFormState::SavingFailed { .. } => vec![Some(Hotkey::new("esc", "return"))],
            SettingsFormState::Saved { .. } => vec![Some(Hotkey::new("esc", "return"))],
        }
        .into_iter()
        .flatten()
        .collect()
    }

    fn render_form(&mut self, id: &FormId, data: &FormData, area: Rect, buf: &mut Buffer) {
        if self.form_list_state.selected.is_none() {
            self.form_list_state.select(Some(0));
        }

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Fill(1)])
            .split(area);

        let v0_h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(2),
                Constraint::Length(2),
            ])
            .split(v[0]);

        Span::from("FIELD").magenta().render(v0_h[1], buf);
        Span::from("VALUE").magenta().render(v0_h[2], buf);
        Span::from("DESCRIPTION").magenta().render(v0_h[3], buf);

        let list_builder = ListBuilder::new(|context| {
            let form_item = &FORMS[id][context.index];

            let item = FormItemWidget {
                form_item,
                value: &data[form_item.key],
                is_selected: context.is_selected,
            };

            (item, 1)
        });

        let list = ListView::new(list_builder, FORMS[id].len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        list.render(v[1], buf, &mut self.form_list_state);
    }
}

impl Component for Settings {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        match event {
            Event::Key(KeyEvent { code, .. }) => match (code, &state.settings_form_state) {
                (KeyCode::Up, SettingsFormState::Inactive) => loop {
                    self.settings_list_state.previous();

                    if self.settings_list_state.selected == Some(0) {
                        break;
                    }

                    if let Some(index) = self.settings_list_state.selected
                        && !matches!(SETTINGS[index], SettingsItem::Group { .. })
                    {
                        break;
                    }
                },
                (KeyCode::Down, SettingsFormState::Inactive) => loop {
                    self.settings_list_state.next();

                    if let Some(index) = self.settings_list_state.selected
                        && !matches!(SETTINGS[index], SettingsItem::Group { .. })
                    {
                        break;
                    }
                },
                (KeyCode::Enter, SettingsFormState::Inactive) => {
                    if let Some(index) = self.settings_list_state.selected
                        && let Some(SettingsItem::Form { id, .. }) = SETTINGS.get(index)
                    {
                        emit(AppEvent::SettingsFormSelected(id.clone()))?;
                    }
                }
                (
                    KeyCode::Esc,
                    SettingsFormState::Loading { .. } | SettingsFormState::LoadingFailed { .. },
                ) => {
                    emit(AppEvent::SettingsFormLoadingCancelRequested)?;
                }
                (KeyCode::Up, SettingsFormState::Loaded { .. }) => {
                    self.form_list_state.previous();
                }
                (KeyCode::Down, SettingsFormState::Loaded { .. }) => {
                    self.form_list_state.next();
                }
                (KeyCode::Esc, SettingsFormState::Loaded { .. }) => {
                    emit(AppEvent::SettingsFormLoadingCancelRequested)?;
                }
                _ => {}
            },
            _ => {}
        }

        Ok(())
    }

    fn render(&mut self, state: &State, frame: &mut Frame, area: Rect) {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(area);

        let v0_h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 4), Constraint::Ratio(3, 4)])
            .split(v[0]);

        if self.settings_list_state.selected.is_none() {
            self.settings_list_state.select(Some(0));
        }

        // Menu
        let menu_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .padding(Padding::symmetric(1, 0))
            .fg(
                if state.settings_form_state == SettingsFormState::Inactive {
                    Color::Yellow
                } else {
                    Color::DarkGray
                },
            );

        let menu_block_area = menu_block.inner(v0_h[0]);

        menu_block.render(v0_h[0], frame.buffer_mut());

        let menu_list_builder = ListBuilder::new(|context| {
            let settings_item = &SETTINGS[context.index];

            let item = SettingsItemWidget {
                settings_item,
                is_selected: context.is_selected,
                is_highlighted: context.is_selected
                    && state.settings_form_state != SettingsFormState::Inactive,
            };

            (item, 1)
        });

        let menu = ListView::new(menu_list_builder, SETTINGS.len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        menu.render(
            menu_block_area,
            frame.buffer_mut(),
            &mut self.settings_list_state,
        );

        // Form
        let form_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(
                if state.settings_form_state == SettingsFormState::Inactive {
                    Color::DarkGray
                } else {
                    Color::Yellow
                },
            ));

        let form_block_area = form_block.inner(v0_h[1]);

        match &state.settings_form_state {
            SettingsFormState::Inactive => {
                PlaceholderWidget::dark_gray("choose the setting")
                    .render(form_block_area, frame.buffer_mut());
            }
            SettingsFormState::Loading { .. } => {
                PlaceholderWidget::dark_gray("loading...")
                    .render(form_block_area, frame.buffer_mut());
            }
            SettingsFormState::LoadingFailed { error, .. } => {
                PlaceholderWidget::red(error).render(form_block_area, frame.buffer_mut());
            }
            SettingsFormState::Loaded { id } => {
                if let Some(data) = &state.settings_form_data {
                    self.render_form(&id, data, form_block_area, frame.buffer_mut());
                }
            }
            _ => {}
        }

        form_block.render(v0_h[1], frame.buffer_mut());

        // Hotkeys
        HotkeysWidget::new(&self.get_hotkeys(&state.settings_form_state))
            .render(v[2], frame.buffer_mut());
    }
}

struct SettingsItemWidget<'a> {
    settings_item: &'a SettingsItem,
    is_selected: bool,
    is_highlighted: bool,
}

impl<'a> Widget for SettingsItemWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new().padding(Padding::right(2));
        let block_area = block.inner(area);

        block.render(area, buf);

        match self.settings_item {
            SettingsItem::Group { title } => {
                Line::from(Span::from(*title))
                    .magenta()
                    .render(block_area, buf);
            }
            SettingsItem::Form { title, .. } => {
                Line::from(vec![
                    if self.is_selected && !self.is_highlighted {
                        Span::from("█ ")
                    } else {
                        Span::from("  ")
                    },
                    Span::from(*title),
                ])
                .fg(if self.is_selected {
                    Color::Yellow
                } else {
                    Color::Reset
                })
                .add_modifier(if self.is_highlighted {
                    Modifier::REVERSED
                } else {
                    Modifier::empty()
                })
                .render(block_area, buf);
            }
        }
    }
}

struct FormItemWidget<'a> {
    form_item: &'a FormItem,
    value: &'a FormValue,
    is_selected: bool,
}

impl<'a> Widget for FormItemWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Fill(2),
                Constraint::Length(2),
            ])
            .split(area);

        // title
        Line::from(
            Span::from(self.form_item.title).add_modifier(if self.is_selected {
                Modifier::UNDERLINED
            } else {
                Modifier::empty()
            }),
        )
        .render(h[1], buf);

        // value
        Line::from(vec![
            Span::from((self.form_item.formatter)(self.value)).patch_style(if self.is_selected {
                Style::new().black().on_yellow()
            } else {
                Style::new()
            }),
        ])
        .render(h[2], buf);

        // description
        Line::from(Span::from(self.form_item.description))
            .dark_gray()
            .render(h[3], buf);
    }
}
