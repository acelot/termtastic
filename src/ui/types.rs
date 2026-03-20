use strum::{Display, EnumCount, EnumIter, FromRepr};

#[derive(Debug, Default, Clone, Copy, PartialEq, Display, FromRepr, EnumIter, EnumCount)]
pub enum Tab {
    #[default]
    #[strum(to_string = "Chat")]
    Chat,
    #[strum(to_string = "Nodes")]
    Nodes,
    #[strum(to_string = "Settings")]
    Settings,
    #[strum(to_string = "Connection")]
    Connection,
    #[strum(to_string = "Logs")]
    Logs,
}

impl Tab {
    pub fn prev(self) -> Self {
        let current_index: usize = self as usize;
        let (previous_index, overflowed) = current_index.overflowing_sub(1);

        Self::from_repr(if overflowed {
            Tab::COUNT - 1
        } else {
            previous_index
        })
        .unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);

        Self::from_repr(if next_index > Tab::COUNT - 1 {
            0
        } else {
            next_index
        })
        .unwrap_or(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hotkey {
    pub key: String,
    pub label: String,
}
