use crate::TelitairoBot;
use teloxide::{prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported"
)]
pub enum AdminCommand {
    #[command(description = "Display this text.")]
    Help,
}

pub async fn handle_admin_commands(
    bot: Bot,
    telitairo_bot: TelitairoBot,
    msg: Message,
    cmd: AdminCommand,
) -> ResponseResult<()> {
    match cmd {
        AdminCommand::Help => {
            bot.send_message(msg.chat.id, AdminCommand::descriptions().to_string())
                .await?;
        }
    };

    Ok(())
}
