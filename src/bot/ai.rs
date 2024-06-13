use crate::{gpt, types, TelitairoBot};
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported"
)]
pub enum AiCommand {
    #[command(description = "Ask the bot a question.")]
    Ask(String),

    #[command(description = "Ask the bot to mediate a discussion")]
    Mediate,
}

pub async fn handle_ai_commands(
    bot: Bot,
    buffer_store: types::BufferStore,
    telitairo_bot: TelitairoBot,
    msg: Message,
    cmd: AiCommand,
) -> ResponseResult<()> {
    match cmd {
        AiCommand::Ask(question) => {
            let answer = match gpt::ask(question, telitairo_bot).await {
                Ok(response) => response,
                Err(err) => format!("Error getting an answer from OpenAI: {err}"),
            };

            bot.send_message(msg.chat.id, answer).await?;
        }
        AiCommand::Mediate => {
            let answer = match gpt::mediate(buffer_store, telitairo_bot, msg.chat.id).await {
                Ok(response) => response,
                Err(err) => format!("Error getting an answer from OpenAI: {err}"),
            };

            bot.send_message(msg.chat.id, answer).await?;
        }
    };

    Ok(())
}
