use telitairos_bot::bot::{handle, Command};
use teloxide::Bot;
use teloxide::repls::CommandReplExt;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");
    
    let telitairo_bot = Bot::from_env();
    Command::repl(telitairo_bot, handle).await;
}
