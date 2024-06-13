use telitairos_bot::TelitairoBot;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot");

    let telitairo_bot = TelitairoBot::default();

    telitairo_bot.dispatch().await;
}
