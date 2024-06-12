use std::collections::HashMap;
use std::sync::Arc;

use teloxide::prelude::*;
use teloxide::{dptree, Bot};
use tokio::sync::RwLock;
pub mod bot;
pub mod gpt;
pub mod types;

pub struct TelitairoBot {}

impl TelitairoBot {
    pub async fn dispatch() {
        let bot = Bot::from_env();
        let buffer: types::Buffer = Arc::new(RwLock::new(HashMap::new()));

        let handler = dptree::entry()
            .branch(
                Update::filter_message()
                    .filter_command::<bot::Command>()
                    .endpoint(bot::handle_commands),
            )
            .branch(Update::filter_message().endpoint(bot::handle_messages));

        Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![buffer])
            .default_handler(|update| async move {
                log::warn!("Unhandled update: {:#?}", update);
            })
            .error_handler(LoggingErrorHandler::with_custom_text(
                "An error occurred in the dispatcher",
            ))
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await
    }
}
