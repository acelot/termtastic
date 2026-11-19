use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ChatPosition {
    pub chat_id: u32,
    pub selected_id: Uuid,
    pub max_id: Uuid,
}
