use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget, Wrap},
};

use crate::{types::Hotkey, ui::widget::HotkeysWidget};

pub struct PopupConfirmWidget {
    text: String,
    width: u16,
}

impl PopupConfirmWidget {
    pub fn new<S: Into<String>>(text: S, width: u16) -> Self {
        Self {
            text: text.into(),
            width,
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

        let popup_area = Rect {
            x: area.x + area.width / 2 - self.width / 2,
            y: area.y + area.height / 2 - height / 2,
            width: self.width,
            height,
        };

        let popup_block = Block::bordered()
            .border_type(BorderType::Thick)
            .border_style(Style::new().yellow())
            .padding(Padding::symmetric(3, 1));

        let popup_block_area = popup_block.inner(popup_area);
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(paragraph_height),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(popup_block_area);

        Clear.render(popup_area, buf);
        popup_block.render(popup_area, buf);

        paragraph.render(v[0], buf);

        HotkeysWidget::new(&vec![Hotkey::new("enter", "yes"), Hotkey::new("esc", "no")])
            .render(v[2], buf);
    }
}
