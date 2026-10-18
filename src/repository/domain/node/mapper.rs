use crate::types::{Node, NodeUser};

impl From<Node> for super::Node {
    fn from(value: Node) -> Self {
        Self::from(&value)
    }
}

impl From<&Node> for super::Node {
    fn from(value: &Node) -> Self {
        let user = value.user.as_ref();

        Self {
            key: value.key,
            hops: value.hops,
            last_heard: value.last_heard,
            snr: value.snr,
            rssi: value.rssi,
            is_favorite: value.is_favorite,
            is_ignored: value.is_ignored,
            is_muted: value.is_muted,
            has_user: user.is_some(),
            user_id: user.map(|u| u.id.clone()).unwrap_or_default(),
            user_short_name: user.map(|u| u.short_name.clone()).unwrap_or_default(),
            user_long_name: user.map(|u| u.long_name.clone()).unwrap_or_default(),
            user_role: user.map(|u| u.role).unwrap_or_default(),
            user_hw_model: user.map(|u| u.hw_model).unwrap_or_default(),
            user_public_key: user.map(|u| u.public_key.clone()).unwrap_or_default(),
            user_is_licensed: user.map(|u| u.is_licensed).unwrap_or_default(),
            user_is_unmessagable: user.map(|u| u.is_unmessagable).unwrap_or_default(),
        }
    }
}

impl From<super::Node> for Node {
    fn from(value: super::Node) -> Self {
        let mut node = Self {
            key: value.key,
            user: value.has_user.then_some(NodeUser {
                id: value.user_id,
                short_name: value.user_short_name,
                long_name: value.user_long_name,
                role: value.user_role,
                hw_model: value.user_hw_model,
                public_key: value.user_public_key,
                is_licensed: value.user_is_licensed,
                is_unmessagable: value.user_is_unmessagable,
            }),
            hops: value.hops,
            last_heard: value.last_heard,
            snr: value.snr,
            rssi: value.rssi,
            is_favorite: value.is_favorite,
            is_ignored: value.is_ignored,
            is_muted: value.is_muted,
            fulltext: String::default(),
        };

        node.update_fulltext();

        node
    }
}
