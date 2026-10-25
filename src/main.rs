mod health;

use telitairos_bot::TelitairoBot;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();
    log::info!("Starting bot");

    let telitairo_bot = TelitairoBot::default();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

    // Keep the endpoint tied to the dispatcher lifetime, rather than a detached task.
    tokio::select! {
        () = telitairo_bot.dispatch() => Ok(()),
        result = health::serve(listener) => result,
    }
}
