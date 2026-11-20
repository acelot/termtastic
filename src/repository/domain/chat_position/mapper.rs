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

impl From<ChatPosition> for (Chat, crate::types::ChatPosition) {
    fn from(val: ChatPosition) -> Self {
        (
            match val.chat_id {
                0..=7 => Chat::Channel(val.chat_id),
                _ => Chat::Direct(val.chat_id),
            },
            crate::types::ChatPosition {
                selected_id: val.selected_id,
                max_id: val.max_id,
            },
        )
    }
}
