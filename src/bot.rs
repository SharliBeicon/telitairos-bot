use std::collections::VecDeque;

use crate::{gpt, types, TelitairoBot};
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported"
)]
///A set of commands supported by the bot.
pub enum Command {
    ///Display this text.
    #[command(description = "Display this text.")]
    Help,

    ///Ask the bot a question.
    #[command(description = "Ask the bot a question.")]
    Ask(String),

    ///Ask the bot to mediate a discussion
    #[command(description = "Ask the bot to mediate a discussion")]
    Mediate,
}

pub async fn handle_commands(
    bot: Bot,
    buffer_store: types::BufferStore,
    telitairo_bot: TelitairoBot,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Ask(question) => {
            let answer = match gpt::ask(question, telitairo_bot).await {
                Ok(response) => response,
                Err(err) => format!("Error getting an answer from OpenAI: {err}"),
            };

            bot.send_message(msg.chat.id, answer).await?;
        }
        Command::Mediate => {
            let answer = match gpt::mediate(buffer_store, telitairo_bot, msg.chat.id).await {
                Ok(response) => response,
                Err(err) => format!("Error getting an answer from OpenAI: {err}"),
            };

            bot.send_message(msg.chat.id, answer).await?;
        }
    };

    Ok(())
}

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
