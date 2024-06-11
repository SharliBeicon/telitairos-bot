use telitairos_bot::{answer, Command};
use teloxide::{prelude::*, repls::CommandReplExt};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot");

    let bot = Bot::from_env();
    Command::repl(bot, answer).await;
}
