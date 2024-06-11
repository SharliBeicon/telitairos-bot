pub mod consts;
pub mod gpt;

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

pub async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
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
        Command::Mediate => {}
    };

    Ok(())
}
