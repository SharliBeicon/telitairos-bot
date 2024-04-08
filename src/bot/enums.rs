use teloxide::utils::command::BotCommands;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
/// A set of commands supported by the bot.
pub enum Command {
    /// Display this text.
    #[command(description = "Display this text.")]
    Help,
}



