pub mod admin;
pub mod ai;

use crate::{types, TelitairoBot};
use std::collections::VecDeque;
pub use teloxide::{prelude::*, utils::command::BotCommands};

pub async fn handle_messages(
    buffer_store: types::BufferStore,
    telitairo_bot: TelitairoBot,
    msg: Message,
) -> ResponseResult<()> {
    let mut buffer_store_lock = buffer_store.write().await;
    match buffer_store_lock.get_mut(&msg.chat.id) {
        Some(buffer) => {
            if buffer.len() == telitairo_bot.buffer_size {
                buffer.pop_front();
            }
            buffer.push_back(msg.clone());
        }
        None => {
            let mut buffer = VecDeque::new();
            buffer.push_back(msg.clone());
            buffer_store_lock.insert(msg.chat.id, buffer);
        }
    }

    Ok(())
}

pub fn all_command_descriptions() -> String {
    let admin_command_descriptions = admin::AdminCommand::descriptions();
    let ai_command_descriptions = ai::AiCommand::descriptions();

    format!("👮🚨{admin_command_descriptions}\n\n\n🦀 🤖{ai_command_descriptions}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_descriptions(){
        assert_eq!(all_command_descriptions(), 
"👮🚨Supported ADMIN Commands

/help — Display this text\\.
/mute — `/mute X {h/m/s/p}` \\-\\> Mute an User from the Chat Group the selected time\\. 'p' is for 'permanent'
/ban — `/ban X {h/m/s/p}` \\-\\> Ban an User from the Chat Group the selected time\\. 'p' is for 'permanent'


🦀 🤖Supported AI Commands

/ask — Ask the bot a question
/mediate — Ask the bot to mediate a discussion"
)
    }
}
