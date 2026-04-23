use tracing_unwrap::OptionExt;

use crate::{
    service::{FORMS, SETTINGS},
    ui::{helpers::default_scrollbar, prelude::*},
};

pub struct Settings<'a> {
    settings_list_state: ListState,
    form_list_state: ListState,
    active_form_item: Option<&'static FormItem>,
    popup_input_state: Option<PopupInputState<'a>>,
    popup_dropdown_state: Option<PopupDropdownState<'a>>,
    is_exit_confirm_visible: bool,
}

impl<'a> Settings<'a> {
    pub fn new() -> Self {
        Self {
            settings_list_state: ListState::default(),
            form_list_state: ListState::default(),
            active_form_item: None,
            popup_input_state: None,
            popup_dropdown_state: None,
            is_exit_confirm_visible: false,
        }
    }

    fn get_hotkeys(&self, state: &State) -> Vec<Hotkey> {
        match &state.settings_form_state {
            SettingsFormState::Inactive => vec![
                Some(Hotkey::new("↑↓", "scroll")),
                Some(Hotkey::new("enter", "open")),
            ],
            SettingsFormState::Loading { .. } => vec![Some(Hotkey::new("esc", "cancel"))],
            SettingsFormState::LoadingFailed { .. } => vec![Some(Hotkey::new("esc", "return"))],
            SettingsFormState::Loaded { .. } if self.active_form_item.is_some() => vec![
                Some(Hotkey::new("enter", "submit")),
                Some(Hotkey::new("esc", "cancel")),
            ],
            SettingsFormState::Loaded { .. } => vec![
                Some(Hotkey::new("↑↓", "scroll")),
                self.form_list_state
                    .selected
                    .is_some()
                    .then_some(Hotkey::new("enter", "edit")),
                state
                    .settings_form_is_changed
                    .then_some(Hotkey::new("s", "save")),
                state
                    .settings_form_is_changed
                    .then_some(Hotkey::new("r", "reset")),
                Some(Hotkey::new("esc", "return")),
            ],
        }
        .into_iter()
        .flatten()
        .collect()
    }

    fn render_form(
        &mut self,
        id: &FormId,
        data: &FormData,
        original_data: &FormData,
        area: Rect,
        buf: &mut Buffer,
    ) {
        if self.form_list_state.selected.is_none() {
            self.form_list_state.select(Some(0));
        }

        let description_paragraph = self
            .form_list_state
            .selected
            .and_then(|index| *&FORMS[id][index].description)
            .and_then(|desc| {
                Some(
                    Paragraph::new(vec![
                        Line::from("DESCRIPTION").magenta(),
                        Line::from(desc).dark_gray(),
                    ])
                    .wrap(Wrap { trim: false }),
                )
            });

        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                vec![
                    Some(Constraint::Length(1)),
                    Some(Constraint::Fill(1)),
                    description_paragraph
                        .is_some()
                        .then_some(Constraint::Length(1)),
                    description_paragraph
                        .as_ref()
                        .and_then(|p| Some(Constraint::Length(p.line_count(area.width) as u16))),
                ]
                .iter()
                .flatten(),
            )
            .split(area);

        let v0_h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(3),
                Constraint::Fill(4),
                Constraint::Length(2),
            ])
            .split(v[0]);

        Span::from("FIELD").magenta().render(v0_h[0], buf);
        Span::from("VALUE").magenta().render(v0_h[1], buf);

        let list_builder = ListBuilder::new(|context| {
            let form_item = &FORMS[id][context.index];

            let item = FormItemWidget {
                form_item,
                value: &data
                    .get(form_item.key)
                    .expect_or_log(&format!("form field not exists: {}", form_item.key)),
                is_selected: context.is_selected,
                is_changed: &data[form_item.key] != &original_data[form_item.key],
            };

            (item, 1)
        });

        let list = ListView::new(list_builder, FORMS[id].len())
            .infinite_scrolling(false)
            .scrollbar(default_scrollbar());

        list.render(v[1], buf, &mut self.form_list_state);

        if let Some(p) = &description_paragraph {
            p.render(v[3], buf);
        }
    }

    fn handle_form_item_edit(
        &mut self,
        form_item: &'static FormItem,
        value: &FormValue,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        match &form_item.kind {
            FormItemKind::InputOfString
            | FormItemKind::InputOfInt32
            | FormItemKind::InputOfUnsignedInt8
            | FormItemKind::InputOfUnsignedInt32
            | FormItemKind::InputOfFloat32 => {
                self.active_form_item = Some(form_item);
                self.popup_input_state = Some(PopupInputState::new(
                    form_item.title,
                    None,
                    value.to_string(),
                ));
            }
            FormItemKind::Enum(variants) => {
                self.active_form_item = Some(form_item);
                self.popup_dropdown_state = Some(PopupDropdownState::new(
                    form_item.title,
                    variants,
                    Some(value.clone()),
                ));
            }
            FormItemKind::Switch => {
                emit(AppEvent::SettingsFormItemSubmitted(
                    form_item,
                    match value {
                        FormValue::Bool(v) => FormValue::Bool(!v),
                        FormValue::Option(Some(b)) if let FormValue::Bool(v) = **b => {
                            FormValue::Option(Some(Box::new(FormValue::Bool(!v))))
                        }
                        FormValue::Option(None) => FormValue::Option(None),
                        _ => unreachable!(),
                    },
                ))?;
            }
            _ => unimplemented!(),
        }

        Ok(())
    }
}

impl<'a> Component for Settings<'a> {
    fn handle_event(
        &mut self,
        state: &State,
        event: &Event,
        emit: &impl Fn(AppEvent) -> anyhow::Result<()>,
    ) -> anyhow::Result<bool> {
        match event {
            Event::Key(KeyEvent { code, kind, .. }) if kind == &KeyEventKind::Press => {
                // Confirm popup
                if self.is_exit_confirm_visible {
                    match code {
                        KeyCode::Enter => {
                            emit(AppEvent::SettingsFormCancelRequested)?;
                            self.is_exit_confirm_visible = false;
                        }
                        KeyCode::Esc => {
                            self.is_exit_confirm_visible = false;
                        }
                        _ => {}
                    }

                    return Ok(true);
                }

                // Input popup
                if let Some(popup_input_state) = self.popup_input_state.as_mut() {
                    let form_item = self.active_form_item.expect("should be Some");

                    match code {
                        KeyCode::Enter => {
                            match handle_popup_input_submit(form_item, popup_input_state) {
                                Ok(value) => {
                                    emit(AppEvent::SettingsFormItemSubmitted(form_item, value))?;
                                    self.active_form_item = None;
                                    self.popup_input_state = None;
                                }
                                Err(e) => {
                                    popup_input_state.set_error(e.to_string());
                                }
                            }
                        }
                        KeyCode::Esc => {
                            self.active_form_item = None;
                            self.popup_input_state = None;
                        }
                        _ => {
                            popup_input_state.handle_event(event.clone());
                        }
                    }

                    return Ok(true);
                }

                // Dropdown popup
                if let Some(popup_dropdown_state) = self.popup_dropdown_state.as_mut()
                    && let Some(value) = popup_dropdown_state.get_value()
                {
                    let form_item = self.active_form_item.expect("should be Some");

                    match code {
                        KeyCode::Enter => {
                            emit(AppEvent::SettingsFormItemSubmitted(
                                form_item,
                                value.clone(),
                            ))?;

                            self.active_form_item = None;
                            self.popup_dropdown_state = None;
                        }
                        KeyCode::Esc => {
                            self.active_form_item = None;
                            self.popup_dropdown_state = None;
                        }
                        _ => {
                            popup_dropdown_state.handle_event(event.clone());
                        }
                    }

                    return Ok(true);
                }

                // Default
                match (code, &state.settings_form_state) {
                    (KeyCode::Up, SettingsFormState::Inactive) => {
                        self.settings_list_state.previous();
                    }
                    (KeyCode::Down, SettingsFormState::Inactive) => {
                        self.settings_list_state.next();
                    }
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
                        emit(AppEvent::SettingsFormCancelRequested)?;
                    }
                    (KeyCode::Up, SettingsFormState::Loaded { .. }) => {
                        self.form_list_state.previous();
                    }
                    (KeyCode::Down, SettingsFormState::Loaded { .. }) => {
                        self.form_list_state.next();
                    }
                    (KeyCode::Enter, SettingsFormState::Loaded { id }) => {
                        if self.is_exit_confirm_visible {
                            emit(AppEvent::SettingsFormCancelRequested)?;
                            self.is_exit_confirm_visible = false;

                            return Ok(true);
                        }

                        let index = self.form_list_state.selected.expect("should be Some");
                        let data = state.settings_form_data.as_ref().expect("should be Some");
                        let form_item = &FORMS[id][index];

                        self.handle_form_item_edit(form_item, &data[form_item.key], emit)?;
                    }
                    (KeyCode::Esc, SettingsFormState::Loaded { .. }) => {
                        if state.settings_form_is_changed {
                            self.is_exit_confirm_visible = true;
                        } else {
                            emit(AppEvent::SettingsFormCancelRequested)?;
                            self.form_list_state = ListState::default();
                        }
                    }
                    (KeyCode::Char('r'), SettingsFormState::Loaded { .. }) => {
                        emit(AppEvent::SettingsFormResetRequested)?;
                    }
                    (KeyCode::Char('s'), SettingsFormState::Loaded { id }) => {
                        emit(AppEvent::SettingsFormSaveRequested(id.clone()))?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(true)
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
                is_implemented: if let SettingsItem::Form { id, .. } = settings_item {
                    FORMS.contains_key(id)
                } else {
                    true
                },
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
            ))
            .padding(Padding::symmetric(1, 0));

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
                let data = state.settings_form_data.as_ref().expect("should be Some");
                let original_data = state
                    .settings_form_original_data
                    .as_ref()
                    .expect("should be Some");

                self.render_form(
                    &id,
                    data,
                    original_data,
                    form_block_area,
                    frame.buffer_mut(),
                );
            }
        }

        form_block.render(v0_h[1], frame.buffer_mut());

        // Hotkeys
        HotkeysWidget::new(&self.get_hotkeys(&state)).render(v[2], frame.buffer_mut());

        // Active input popup
        if let Some(state) = self.popup_input_state.as_mut() {
            PopupInputWidget::new(40).render(form_block_area, frame.buffer_mut(), state);
        }

        // Active dropdown popup
        if let Some(state) = self.popup_dropdown_state.as_mut() {
            PopupDropdownWidget::new(40).render(form_block_area, frame.buffer_mut(), state);
        }

        // Confirm popup
        if self.is_exit_confirm_visible {
            PopupConfirmWidget::new(
                "There are unsaved settings, do you want to reset the fields?",
                36,
            )
            .render(form_block_area, frame.buffer_mut());
        }
    }
}

fn handle_popup_input_submit<'a>(
    form_item: &'static FormItem,
    input_state: &mut PopupInputState<'a>,
) -> anyhow::Result<FormValue> {
    let input_value = input_state.get_value();

    match form_item.kind {
        FormItemKind::InputOfString => {
            let value = FormValue::from(input_value);
            (form_item.validator)(&value).and_then(|_| Ok(value))
        }
        FormItemKind::InputOfInt32 => {
            let value = FormValue::from(input_value.parse::<i32>()?);
            (form_item.validator)(&value).and_then(|_| Ok(value))
        }
        FormItemKind::InputOfUnsignedInt8 => {
            let value = FormValue::from(input_value.parse::<u8>()?);
            (form_item.validator)(&value).and_then(|_| Ok(value))
        }
        FormItemKind::InputOfUnsignedInt32 => {
            let value = FormValue::from(input_value.parse::<u32>()?);
            (form_item.validator)(&value).and_then(|_| Ok(value))
        }
        FormItemKind::InputOfFloat32 => {
            let value = FormValue::from(input_value.parse::<f32>()?);
            (form_item.validator)(&value).and_then(|_| Ok(value))
        }
        _ => unimplemented!(),
    }
}

struct SettingsItemWidget<'a> {
    settings_item: &'a SettingsItem,
    is_selected: bool,
    is_highlighted: bool,
    is_implemented: bool,
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
                Line::from(
                    Span::from(*title)
                        .magenta()
                        .add_modifier(if self.is_selected {
                            Modifier::REVERSED
                        } else {
                            Modifier::empty()
                        }),
                )
                .render(block_area, buf);
            }
            SettingsItem::Form { title, .. } => {
                Line::from(vec![
                    if self.is_selected && !self.is_highlighted {
                        Span::from("█ ")
                    } else {
                        Span::from("  ")
                    },
                    Span::from(*title).add_modifier(if !self.is_implemented {
                        Modifier::DIM
                    } else {
                        Modifier::empty()
                    }),
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
    form_item: &'static FormItem,
    value: &'a FormValue,
    is_selected: bool,
    is_changed: bool,
}

impl<'a> Widget for FormItemWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(3),
                Constraint::Fill(4),
                Constraint::Length(2),
            ])
            .split(area);

        // title
        Line::from(
            Span::from(self.form_item.title)
                .add_modifier(if self.is_selected {
                    Modifier::UNDERLINED | Modifier::BOLD
                } else {
                    Modifier::empty()
                })
                .fg(if self.is_changed {
                    Color::Cyan
                } else {
                    Color::Reset
                }),
        )
        .render(h[0], buf);

        // value
        let formatted_value = if matches!(self.form_item.kind, FormItemKind::Switch) {
            let value = match self.value {
                FormValue::Bool(v) => *v,
                FormValue::Option(Some(b)) => b.as_bool().expect("invalid FormValue"),
                _ => unreachable!(),
            };

            if value == true {
                "[✔]".to_owned()
            } else {
                "[ ]".to_owned()
            }
        } else if self.form_item.kind.is_enum() {
            format!("{} ↓", (self.form_item.formatter)(self.value))
        } else {
            (self.form_item.formatter)(self.value)
        };

        Line::from(Span::from(formatted_value).patch_style(
            match (self.is_selected, self.is_changed) {
                (true, true) => Style::new().white().on_cyan(),
                (true, false) => Style::new().black().on_yellow(),
                (false, true) => Style::new().cyan(),
                _ => Style::new(),
            },
        ))
        .render(h[1], buf);
    }
}
