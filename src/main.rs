use std::{collections::HashMap, sync::Arc};
use telitairos_bot::{bot, types};
use teloxide::prelude::*;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot");

    let bot = Bot::from_env();

    let messages_store: types::Messages = Arc::new(RwLock::new(HashMap::new()));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<bot::Command>()
                .endpoint(bot::handle_commands),
        )
        .branch(Update::filter_message().endpoint(bot::handle_messages));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![messages_store])
        .default_handler(|update| async move {
            log::warn!("Unhandled update: {:#?}", update);
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
