use crate::{types::Hotkey, ui::widget::HotkeysWidget};
use ratatui::style::Color;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget, Wrap},
};

pub struct PopupConfirmWidget {
    text: String,
    yes: String,
    no: String,
    width: u16,
    border_color: Color,
}

impl PopupConfirmWidget {
    pub fn new<S: Into<String>>(text: S, yes: S, no: S, width: u16, border_color: Color) -> Self {
        Self {
            text: text.into(),
            yes: yes.into(),
            no: no.into(),
            width,
            border_color,
        }
    }

    #[allow(unused)]
    pub fn yes_no<S: Into<String>>(text: S, width: u16) -> Self {
        Self {
            text: text.into(),
            yes: "yes".to_owned(),
            no: "no".to_owned(),
            width,
            border_color: Color::Yellow,
        }
    }
}

impl Widget for PopupConfirmWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let paragraph = Paragraph::new(self.text).wrap(Wrap { trim: false });
        let paragraph_height = paragraph.line_count(self.width - 8) as u16;
        let height = paragraph_height + 6;
        let popup_area = area.centered(Constraint::Length(self.width), Constraint::Length(height));

        let popup_block = Block::bordered()
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(self.border_color))
            .padding(Padding::symmetric(3, 1));

        let popup_block_area = popup_block.inner(popup_area);
        let v = Layout::vertical([
            Constraint::Length(paragraph_height),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(popup_block_area);

        Clear.render(popup_area.outer(Margin::new(1, 0)), buf);
        popup_block.render(popup_area, buf);

        paragraph.render(v[0], buf);

        HotkeysWidget::new(&vec![Hotkey::new("enter", &self.yes), Hotkey::new("esc", &self.no)]).render(v[2], buf);
    }
}
