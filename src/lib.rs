//! # A fully functional AI Powered Telegram bot
//!
//! ## Supported commands
//! You can either:
//! - `/ask` for a specified question.
//! - `/mediate` to read the last N messages of a chat group and mitigate an argument.
//!
//! ## Environment variables needed
//!
//! ```bash
//! - TELOXIDE_TOKEN= "/* Your Telegram Bot API Key */"
//! - OPENAI_API_KEY= "/* Your OpenAI API Key */"
//! - OPENAI_ORG_ID= "/* Your OpenAI Organization ID */"
//! ```
//!
//! ## Basic usage
//!
//! You need to specify the personality of the bot as well as its criteria when mitigating an argument.
//! A size for the context of the N last messages of the chat group is also needed.
//!
//! For a detailed example go to [TelitairoBot]
//!
//! ### Example
//!
//! TelitairoBot struct implements Default trait, so you can start a bot with a generic personality
//! by just doing this:
//!
//! ```
//! # use telitairos_bot::TelitairoBot;
//! let telitairo_bot: TelitairoBot = Default::default();
//! ```
//!
//! But if you want to set your own Bot's personality you can use the `new()` function like this:
//!
//! ```
//! # use telitairos_bot::TelitairoBot;
//! /* #[tokio::main] */
//! /*async*/ fn main() {
//! let telitairo_bot = TelitairoBot::new(
//!     String::from("personality"),
//!     String::from("mediation criteria"),
//!     200 /* buffer size */,
//! );
//!
//! /* telitairo_bot.dispatch().await; */
//! # }
//! ```
//!
mod bot;
mod gpt;
mod types;

use crate::bot::*;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::{dptree, Bot};
use tokio::sync::RwLock;

/// Defines the bot behavior
#[derive(Clone, Debug, PartialEq)]
pub struct TelitairoBot {
    /// String to define the bot personality, a descriptive short prompt.
    ///
    /// # Example
    /// ```bash
    /// "You are a virtual assistant with a touch of acid humour and you love potatoes"
    /// ```
    pub personality: String,
    /// String to define the bot action when `/mediate` command is sent. descriptive short prompt.
    ///
    /// # Example
    /// ```bash
    /// "Take the messages, search for possible discussions and choose one side"
    /// ```
    pub mediate_query: String,
    /// Buffer capacity, size of the context for the `/mediate` command.
    ///
    /// Bot will assert if buffer_size = 0
    pub buffer_size: usize,
}

impl TelitairoBot {
    /// Creates a new bot with the selected parameters
    ///
    /// Creation will assert if strings are empty or if buffer_size = 0
    pub fn new(personality: String, mediate_query: String, buffer_size: usize) -> TelitairoBot {
        assert!(buffer_size > 0 && !personality.is_empty() && !mediate_query.is_empty());

        TelitairoBot {
            personality,
            mediate_query,
            buffer_size,
        }
    }

    /// Open a listening for incoming messages and commands
    pub async fn dispatch(&self) {
        let bot = Bot::from_env();
        let buffer_store: types::BufferStore = Arc::new(RwLock::new(HashMap::new()));

        let handler = Update::filter_message()
            .branch(
                dptree::entry()
                    .filter_command::<ai::AiCommand>()
                    .endpoint(ai::handle_ai_commands),
            )
            .branch(
                dptree::entry()
                    .filter_command::<admin::AdminCommand>()
                    .endpoint(admin::handle_admin_commands),
            )
            .branch(dptree::endpoint(handle_messages));

        Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![buffer_store, self.clone()])
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

/// Default attributes for a generic assistant bot
impl Default for TelitairoBot {
    fn default() -> Self {
        TelitairoBot {
            personality: types::DEFAULT_PERSONALITY.to_string(),
            mediate_query: types::DEFAULT_MEDIATION_QUERY.to_string(),
            buffer_size: types::DEFAULT_BUFFER_SIZE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_and_new() {
        let telitairo_default = TelitairoBot::default();
        let telitairo_partially_default = TelitairoBot {
            buffer_size: 200,
            ..Default::default()
        };
        let telitairo_new = TelitairoBot::new(
            String::from(
                "You are a virtual assistant with a touch of acid humour and you love potatoes",
            ),
            String::from("Take the messages, search for possible discussions and choose one side"),
            200,
        );

        assert_eq!(telitairo_default, telitairo_new);
        assert_eq!(telitairo_partially_default, telitairo_new);
    }
}
