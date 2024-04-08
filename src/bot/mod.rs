pub mod enums;
use crate::bot::enums::Command;
use teloxide::{prelude::*, utils::command::BotCommands};

pub struct TelitairoBot{
    pub bot: Bot,
}

impl TelitairoBot{
    pub fn new() -> Self {
        Self { bot: Bot::from_env() } 
    }

    pub async fn run(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()>{
        match cmd {
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
            }
        }
       Ok(()) 
    }
}

impl Default for TelitairoBot {
    fn default() -> Self {
        Self::new()
    }
}

