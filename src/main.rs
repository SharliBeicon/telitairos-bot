use telitairos_bot::bot::{enums::Command, TelitairoBot};
use teloxide::repls::CommandReplExt;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let telitairo = TelitairoBot::new();

    Command::repl(telitairo.bot, TelitairoBot::run).await;
}
