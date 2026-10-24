//! Limit access to allowed groups before processing commands or collecting history.

use crate::types::BufferStore;
use std::{collections::HashSet, env, sync::Arc};
use teloxide::{
    prelude::*,
    types::{Chat, ChatMemberUpdated},
};

#[derive(Clone)]
pub(crate) struct GroupAllowlist(Arc<HashSet<ChatId>>);

impl GroupAllowlist {
    pub(crate) fn parse(raw: &str) -> Result<Self, &'static str> {
        let mut groups = HashSet::new();
        for token in raw.split(',').map(str::trim) {
            let digits = token.strip_prefix('-').ok_or(
                "ALLOWED_GROUP_IDS must contain comma-separated negative numeric group IDs",
            )?;
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(
                    "ALLOWED_GROUP_IDS must contain comma-separated negative numeric group IDs",
                );
            }
            let id = token
                .parse::<i64>()
                .map_err(|_| "ALLOWED_GROUP_IDS contains an out-of-range group ID")?;
            if id >= 0 {
                return Err("ALLOWED_GROUP_IDS cannot contain zero or private-chat IDs");
            }
            groups.insert(ChatId(id));
        }
        if groups.is_empty() {
            return Err("ALLOWED_GROUP_IDS must contain at least one group ID");
        }
        Ok(Self(Arc::new(groups)))
    }

    pub(crate) fn from_env() -> Result<Self, &'static str> {
        Self::parse(
            &env::var("ALLOWED_GROUP_IDS")
                .map_err(|_| "ALLOWED_GROUP_IDS must be set and nonempty")?,
        )
    }

    fn allows(&self, chat: &Chat) -> bool {
        (chat.is_group() || chat.is_supergroup()) && self.0.contains(&chat.id)
    }
}

async fn reject(bot: &Bot, store: &BufferStore, chat: &Chat, present: bool) {
    store.write().await.remove(&chat.id);
    if present && !chat.is_private() && bot.leave_chat(chat.id).await.is_err() {
        // If the bot cannot leave, keep blocking access. Retry when another update arrives.
        log::warn!("Could not leave an unauthorized chat; its updates remain blocked");
    }
}

pub(crate) async fn allow_message(
    bot: Bot,
    allowed_groups: GroupAllowlist,
    buffer_store: BufferStore,
    msg: Message,
) -> bool {
    if allowed_groups.allows(&msg.chat) {
        return true;
    }
    reject(&bot, &buffer_store, &msg.chat, true).await;
    false
}

pub(crate) async fn handle_membership(
    bot: Bot,
    allowed_groups: GroupAllowlist,
    buffer_store: BufferStore,
    update: ChatMemberUpdated,
) -> ResponseResult<()> {
    if !allowed_groups.allows(&update.chat) {
        reject(
            &bot,
            &buffer_store,
            &update.chat,
            update.new_chat_member.kind.is_present(),
        )
        .await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{tests::message, BufferedMessage};
    use serde_json::json;
    use std::collections::{HashMap, VecDeque};
    use tokio::sync::RwLock;

    #[test]
    fn parses_explicit_negative_group_ids_and_deduplicates() {
        let groups = GroupAllowlist::parse(" -123, -1009876543210, -123 ").unwrap();
        assert_eq!(groups.0.len(), 2);
        assert!(groups.0.contains(&ChatId(-123)));
        assert!(groups.0.contains(&ChatId(-1009876543210)));
    }

    #[test]
    fn rejects_empty_malformed_and_private_chat_configuration() {
        for raw in [
            "",
            " ",
            "*",
            "all",
            "123",
            "0",
            "-0",
            "-",
            "-123,",
            ",-123",
            "-123,, -456",
            "-123 -456",
            "-123,456",
            "@group",
            "-1.5",
            "-１２３",
            "-99999999999999999999999999",
        ] {
            assert!(GroupAllowlist::parse(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn only_listed_groups_and_supergroups_are_allowed() {
        let groups = GroupAllowlist::parse("-123,-1009876543210").unwrap();
        for (id, kind, expected) in [
            (-123i64, "group", true),
            (-1009876543210, "supergroup", true),
            (-456, "supergroup", false),
            (123, "private", false),
            (-123, "channel", false),
        ] {
            let chat: Chat = serde_json::from_value(
                json!({"id":id,"type":kind,"title":"Group","first_name":"User"}),
            )
            .unwrap();
            assert_eq!(groups.allows(&chat), expected, "{id} {kind}");
        }
    }

    #[tokio::test]
    async fn private_commands_are_blocked_and_history_is_removed_without_network() {
        let msg = message(
            json!({"text":"/ask hello","chat":{"id":123,"type":"private","first_name":"User"}}),
        );
        let store: BufferStore = Arc::new(RwLock::new(HashMap::from([(
            msg.chat.id,
            VecDeque::from([BufferedMessage {
                author: "A".into(),
                text: "old".into(),
                sources: vec![],
            }]),
        )])));
        assert!(
            !allow_message(
                Bot::new("offline-placeholder"),
                GroupAllowlist::parse("-123").unwrap(),
                store.clone(),
                msg
            )
            .await
        );
        assert!(store.read().await.is_empty());
    }

    #[tokio::test]
    async fn authorized_commands_pass_the_gate_without_network() {
        let msg = message(json!({"text":"/summary"}));
        let store: BufferStore = Arc::new(RwLock::new(HashMap::new()));
        assert!(
            allow_message(
                Bot::new("offline-placeholder"),
                GroupAllowlist::parse("-1001").unwrap(),
                store,
                msg
            )
            .await
        );
    }

    #[tokio::test]
    async fn departure_events_do_not_trigger_another_leave_request() {
        let update: ChatMemberUpdated = serde_json::from_value(json!({
            "chat":{"id":-456,"type":"supergroup","title":"Group"},
            "from":{"id":1,"is_bot":false,"first_name":"User"},"date":0,
            "old_chat_member":{"user":{"id":2,"is_bot":true,"first_name":"Bot"},"status":"member"},
            "new_chat_member":{"user":{"id":2,"is_bot":true,"first_name":"Bot"},"status":"left"}
        }))
        .unwrap();
        let store: BufferStore = Arc::new(RwLock::new(HashMap::new()));
        handle_membership(
            Bot::new("offline-placeholder"),
            GroupAllowlist::parse("-123").unwrap(),
            store,
            update,
        )
        .await
        .unwrap();
    }
}
