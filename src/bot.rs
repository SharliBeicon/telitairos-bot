use std::collections::VecDeque;

use crate::{gpt, types};
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
    messages: types::Messages,
    msg: Message,
    cmd: Command,
) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
        Command::Ask(question) => {
            let answer = match gpt::ask(question).await {
                Ok(response) => response,
                Err(err) => format!("Error getting an answer from OpenAI: {err}"),
            };

            bot.send_message(msg.chat.id, answer).await?;
        }
        Command::Mediate => {
            let answer = match gpt::mediate(messages, msg.chat.id).await {
                Ok(response) => response,
                Err(err) => format!("Error getting an answer from OpenAI: {err}"),
            };

            bot.send_message(msg.chat.id, answer).await?;
        }
    };

    Ok(())
}

pub async fn handle_messages(messages: types::Messages, msg: Message) -> ResponseResult<()> {
    let mut messages_lock = messages.write().await;
    match messages_lock.get_mut(&msg.chat.id) {
        Some(buffer) => {
            if buffer.len() == types::STORE_CAPACITY {
                buffer.pop_front();
            }
            buffer.push_back(msg.clone());
        }
        None => {
            let mut buffer = VecDeque::new();
            buffer.push_back(msg.clone());
            messages_lock.insert(msg.chat.id, buffer);
        }
    }

    Ok(())
}
