use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Padding},
};

pub struct UnreadMessagesWidget {
    unread: usize,
    dim: bool,
}

impl UnreadMessagesWidget {
    pub fn new(unread: usize) -> Self {
        Self { unread, dim: false }
    }

    pub fn dim(mut self, dim: bool) -> Self {
        self.dim = dim;
        self
    }

    pub fn width(&self) -> u16 {
        self.unread.checked_ilog10().unwrap_or(0) as u16 + 15
    }

    pub fn height(&self) -> u16 {
        3
    }
}

impl Widget for UnreadMessagesWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let unread_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::new().blue())
            .padding(Padding::symmetric(1, 0))
            .add_modifier(if self.dim { Modifier::DIM } else { Modifier::empty() });

        let unread_block_area = unread_block.inner(area);
        unread_block.render(area, buf);

        let unread_span = Span::from(format!("↓ unread: {}", self.unread)).blue();
        unread_span.render(unread_block_area, buf);
    }
}
