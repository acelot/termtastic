use crate::{repository::domain::chat_position::ChatPosition, types::Chat};

impl From<(Chat, crate::types::ChatPosition)> for ChatPosition {
    fn from((chat, chat_position): (Chat, crate::types::ChatPosition)) -> Self {
        Self {
            chat_id: match chat {
                Chat::Channel(channel) => channel,
                Chat::Direct(node_key) => node_key,
            },
            selected_id: chat_position.selected_id,
            max_id: chat_position.max_id,
        }
    }
}

impl Into<(Chat, crate::types::ChatPosition)> for ChatPosition {
    fn into(self) -> (Chat, crate::types::ChatPosition) {
        (
            match self.chat_id {
                0..=7 => Chat::Channel(self.chat_id),
                _ => Chat::Direct(self.chat_id),
            },
            crate::types::ChatPosition {
                selected_id: self.selected_id,
                max_id: self.max_id,
            },
        )
    }
}
