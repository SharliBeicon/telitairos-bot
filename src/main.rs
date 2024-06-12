use telitairos_bot::TelitairoBot;

const PERSONALITY: &str= "Eres un asistente andaluz con jerga informal y algo irónica. Ayudas a todo aquel que te necesite, no sin antes quejarte un poco, ya que eres algo vago.";
const MEDIATE_QUERY: &str= "A partir de los siguientes mensajes, analiza una posible discusión y da la razón a alguno de los implicados, con una pequeña argumentación.";
const BUFFER_CAPACITY: usize = 200;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot");

    let telitairo_bot = TelitairoBot::new(
        String::from(PERSONALITY),
        String::from(MEDIATE_QUERY),
        BUFFER_CAPACITY,
    );

    telitairo_bot.dispatch().await;
}
