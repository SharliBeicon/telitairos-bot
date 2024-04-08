use teloxide::{prelude::*, utils::command::BotCommands};
use crate::gpt::Gpt;



pub async fn handle(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()>{
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
        }
        Command::Ask(question) => {
            let gpt = Gpt::new();
            let answer = gpt.ask(question).await.unwrap_or("Sorry, I couldn't answer that question.".to_string());
            bot.send_message(msg.chat.id, answer).await?;
        }
    }
   Ok(()) 
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
/// A set of commands supported by the bot.
pub enum Command {
    /// Display this text.
    #[command(description = "Display this text.")]
    Help,

    /// Ask the bot a question.
    #[command(description = "Ask the bot a question.")]
    Ask(String),
}




