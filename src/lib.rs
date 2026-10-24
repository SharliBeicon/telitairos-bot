//! # Telitairos Bot 0.2.0
//!
//! A Telegram assistant for members of allowed groups.
//! Members can use `/ask <question>`, `/mediate [N] [goal]`, `/summary [N]`, and `/help`.
//! Mediation accepts a count, a goal, both, or neither.
//! For example, `/mediate 50 Ayúdanos a acordar un plazo realista` sets both.
//! Summaries accept only an optional count and cover facts, key points, decisions, and open questions.
//! Facts take priority over personality.
//!
//! The buffer is the message history that the bot keeps in memory.
//! Mediation and summaries use all available buffered history by default.
//! `N` selects the newest available messages and must be an integer from 1 to `buffer_size`.
//! If fewer messages are available, the bot uses all available messages.
//! It rejects invalid counts, including negative, malformed, and too large numbers.
//! A first word that starts with a digit or sign counts as `N`.
//!
//! If a mediation goal starts with a number, put a word before that number.
//! Goals add to the configured mediation criteria without replacing them.
//! Valid AI requests send a processing notice before downloads or model calls.
//! The bot replaces the notice with the result or an operation error.
//! Long results continue in new messages.
//! Notices reply to the command, or to the original audio for `/transcript`.
//!
//! If an edit fails, the bot sends new result messages and tries to delete the notice.
//! Telegram request limits can stop delivery. If the initial notice fails, the bot stops paid processing.
//! Chat AI commands read shared public URLs, images, PDF and UTF-8 text files, and Telegram audio through transcription.
//! `/ask` also reads the message that you reply to.
//! The bot stores resource references without downloading their contents.
//!
//! Each command attempts up to eight different resources when needed, within 180 seconds before the answer.
//! Downloads have limits of 20 MB from Telegram and 2 MB from the web.
//! Extracted text has limits of 6000 characters per resource and 24,000 per command.
//! The final context has a 50,000-character limit.
//! The bot adds notices for failed readings and detected omissions.
//!
//! The bot treats external content as untrusted information.
//! It blocks private and reserved networks, examines redirects, and uses only approved DNS addresses.
//! DNS maps hostnames to IP addresses.
//! The bot does not run JavaScript or analyze video.
//!
//! Administrators with restrict-members permission can reply to a user message with `/mute N h/m/s/p` or `/ban N h/m/s/p`.
//! Muting and temporary bans require a supergroup. Permanent bans also work in basic groups.
//! `N` is an integer from 0 to 255. The unit `p` means permanent and ignores `N`.
//! Temporary durations must be at least 60 seconds to allow for delays above the Telegram 30-second permanent-action threshold.
//! The bot rejects anonymous administrator commands. The bot also needs administrator restrict-members permission.
//!
//! `/unban` and `/unmute` take no arguments and work only in supergroups.
//! They use the same administrator and reply-target rules as mute and ban.
//! `/unban` lets a banned user rejoin through an invitation or link.
//! It never kicks current members or adds users automatically.
//! `/unmute` restores a restricted current member to the default supergroup permissions, not unrestricted permissions.
//! If the bot cannot read those defaults, it denies the action.
//!
//! Transcription converts speech to text. Reply to an audio or voice message with `/transcript`, without arguments.
//! The bot sends the audio to `gpt-transcribe`, independent of `OPENAI_MODEL`.
//! Each result message replies to the original audio, in its topic when applicable.
//! If someone deletes the original message, the bot can deliver the result without a reply link.
//! The result keeps the spoken language without personality, translation, or Spanish chat instructions.
//! Model errors remain possible.
//!
//! Only Telegram audio and voice messages are supported, not documents or video.
//! Formats include FLAC, MP3, MP4, MPEG, MPGA, M4A, OGG (including voice Opus), WAV, and WebM, subject to provider support.
//! These formats remain untested with live requests.
//! A MIME type identifies a file format. Voice MIME types also select the format, including MP3 and M4A.
//! If voice MIME information is absent or generic, the bot uses OGG.
//! It rejects unsupported MIME types.
//!
//! Downloads have a 20,000,000-byte limit and a shared 60-second deadline for file information and download requests.
//! The bot examines file information and bytes as they arrive.
//! The default Telegram HTTP client has a shorter 17-second timeout per request and a 5-second connection timeout.
//! It can stop the download earlier. Audio stays in memory without temporary files or logs.
//! Transcription has a separate 120-second deadline, including retries by the SDK (software development kit).
//!
//! There is no audio duration limit or duration quota.
//! The bot processes updates one at a time in each chat.
//! Transcription can delay later commands and messages in that chat, including moderation.
//!
//! ## Configuration and privacy
//!
//! Set `TELOXIDE_TOKEN`, `OPENAI_API_KEY`, and `ALLOWED_GROUP_IDS` in the bot process environment.
//! `ALLOWED_GROUP_IDS` is a nonempty comma-separated list of negative group IDs.
//! The bot examines access before commands and history collection. It allows only listed groups and supergroups.
//! It ignores private messages and rejects channels.
//! When an unauthorized group sends a membership update or message, the bot tries to leave and clears its buffered history.
//! If the bot cannot leave, it still blocks access. Missing or invalid configuration prevents startup.
//! The bot examines existing groups when their next update arrives.
//! After you change the list, restart the bot. If a group becomes a supergroup, add its new ID explicitly.
//! Bot replies and default prompts are in Spanish, including help and error feedback.
//! A common language instruction also applies to custom personalities.
//! `OPENAI_ORG_ID` is optional. `OPENAI_MODEL` defaults to `gpt-6.1-sol` and must support Chat Completions.
//! It must also support image and PDF inputs to read those resources.
//! Final chat answers have a 60-second deadline after resource reading. Never log credentials.
//! Only the exact default model receives the low reasoning effort parameter. Other model names omit it.
//! Tokens are units of text that models process.
//! Completion limits are 4096 tokens for questions and 8192 for history commands, including reasoning tokens.
//!
//! The default personality uses dry sarcasm and affectionate teasing among friends.
//! It allows brief rants but puts facts, evidence, and admitted uncertainty before jokes or agreement.
//! Mediation represents positions fairly without treating all claims as equally supported.
//! It can draw firm conclusions from evidence or propose practical agreements.
//! Summaries use the configured voice throughout, not only in a final joke.
//! With the default personality, this includes dry humor and affectionate sarcasm.
//!
//! Summaries never invent events or attribute the bot commentary to participants.
//! They do not use mediation criteria or request-specific goals.
//!
//! Chat AI commands can send images, PDF files, audio, and extracted website or file text to OpenAI.
//! Mediation and summaries also send author names. Repeated commands read resources again and can incur additional charges.
//! Website operators see requests from the bot server. Inform participants before use.
//! `/transcript` sends the selected audio to OpenAI and can incur costs.
//! Inform the speaker and participants before you use it.
//!
//! To receive ordinary group messages, make the bot an administrator or disable Telegram privacy mode.
//! Telegram does not deliver messages from other bots.
//! The bot only knows messages delivered while it runs. It never fetches Telegram history.
//!
//! Each chat shares one in-memory history across its topics.
//! It retains the newest `buffer_size` supported messages that the bot does not handle as commands.
//! Restart clears it. Entries and history text have a 50,000-Unicode-character limit.
//! The bot quotes fields and escapes special characters. It keeps the newest whole messages.
//! If the newest message exceeds the limit, the bot keeps its author and newest text.
//! There is no expiry, total chat limit, or spending quota.
//!
//! All members of allowed groups can use AI commands. Monitor costs.
//! Chat AI answers use basic Markdown for bold, italic, links, and code.
//! Telegram entities mark the formatted text ranges. The bot adjusts these ranges for each message part.
//! Raw HTML stays literal. Transcriptions and status messages stay in plain text.
//!
//! Replies split at 4096 UTF-16 units. The bot skips parts that contain only whitespace.
//! It sends replies in the original topic only for actual topic messages.
//! Retries after Telegram request limits wait at most 30 seconds.
//!
//! ## Basic usage
//!
//! ```
//! use telitairos_bot::TelitairoBot;
//! let bot = TelitairoBot::default();
//! assert_eq!(bot.buffer_size, 500);
//! ```
//!
//! ```no_run
//! use telitairos_bot::TelitairoBot;
//!
//! #[tokio::main]
//! async fn main() {
//!     let bot = TelitairoBot::new(
//!         "Eres un asistente tranquilo y útil. Responde en español.".to_owned(),
//!         "Expón los desacuerdos con imparcialidad y propone próximos pasos constructivos.".to_owned(),
//!         200,
//!     );
//!     bot.dispatch().await;
//! }
//! ```
//!
mod access;
mod bot;
mod content;
mod gpt;
mod types;

use crate::bot::*;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::{dptree, Bot};
use tokio::sync::RwLock;

/// Defines the bot personality, mediation criteria, and history capacity.
#[derive(Clone, Debug, PartialEq)]
pub struct TelitairoBot {
    /// Sets the voice for chat AI commands. Summaries put facts first. Transcription ignores this field.
    ///
    /// # Example
    /// ```bash
    /// "Eres un asistente cercano, útil y respetuoso. Responde en español."
    /// ```
    pub personality: String,
    /// Sets the instructions that the bot uses for `/mediate`.
    ///
    /// # Example
    /// ```bash
    /// "Facilita un acuerdo neutral y propone próximos pasos concretos sin inventar pruebas."
    /// ```
    pub mediate_query: String,
    /// Sets the maximum number of saved messages for `/mediate` and `/summary`.
    ///
    /// `new` and `dispatch` panic if `buffer_size` is zero.
    pub buffer_size: usize,
}

impl TelitairoBot {
    /// Creates a bot with the selected personality, mediation criteria, and history capacity.
    ///
    /// # Panics
    /// Panics if either string is empty or `buffer_size` is zero.
    pub fn new(personality: String, mediate_query: String, buffer_size: usize) -> TelitairoBot {
        assert!(buffer_size > 0 && !personality.is_empty() && !mediate_query.is_empty());

        TelitairoBot {
            personality,
            mediate_query,
            buffer_size,
        }
    }

    /// Listens for incoming messages and commands.
    ///
    /// # Panics
    /// Panics if `buffer_size` is zero or required environment variables are missing, empty, or invalid.
    /// Also panics if the Telegram dispatcher cannot start.
    pub async fn dispatch(&self) {
        assert!(
            self.buffer_size > 0,
            "buffer_size must be greater than zero"
        );
        let allowed_groups =
            access::GroupAllowlist::from_env().expect("Invalid ALLOWED_GROUP_IDS configuration");
        assert!(
            std::env::var("TELOXIDE_TOKEN").is_ok_and(|token| !token.trim().is_empty()),
            "TELOXIDE_TOKEN must be set and nonempty"
        );
        let ai = gpt::Ai::from_env();
        let bot = Bot::from_env();
        self.dispatcher_builder(bot, ai, allowed_groups)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await
    }

    fn dispatcher_builder(
        &self,
        bot: Bot,
        ai: gpt::Ai,
        allowed_groups: access::GroupAllowlist,
    ) -> teloxide::dispatching::DispatcherBuilder<
        Bot,
        teloxide::RequestError,
        teloxide::dispatching::DefaultKey,
    > {
        let buffer_store: types::BufferStore = Arc::new(RwLock::new(HashMap::new()));

        let messages = Update::filter_message()
            .filter_async(access::allow_message)
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

        let handler = dptree::entry()
            .branch(Update::filter_my_chat_member().endpoint(access::handle_membership))
            .branch(messages)
            .branch(Update::filter_channel_post().filter_async(access::allow_message));

        Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![
                buffer_store,
                self.clone(),
                ai,
                allowed_groups
            ])
            .default_handler(|_| async move {
                log::debug!("Ignoring unsupported update");
            })
            .error_handler(Arc::new(|_: teloxide::RequestError| async {
                log::warn!("Telegram handler request failed");
            }))
    }
}

/// Provides the default sarcastic personality, factual mediation criteria, and history capacity.
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

    #[tokio::test]
    async fn dispatcher_dependencies_validate_without_network() {
        // build() makes sure that each handler receives the required dependencies without starting the bot.
        let _dispatcher = TelitairoBot::default()
            .dispatcher_builder(
                Bot::new("offline-placeholder"),
                gpt::Ai::fixture(),
                access::GroupAllowlist::parse("-1001").unwrap(),
            )
            .build();
    }

    #[test]
    #[should_panic]
    fn new_rejects_empty_instructions() {
        TelitairoBot::new(String::new(), "criteria".to_owned(), 1);
    }

    #[test]
    #[should_panic]
    fn new_rejects_zero_capacity() {
        TelitairoBot::new("personality".to_owned(), "criteria".to_owned(), 0);
    }

    #[tokio::test]
    #[should_panic(expected = "buffer_size must be greater than zero")]
    async fn dispatch_rejects_zero_capacity_before_credentials_or_network() {
        TelitairoBot {
            buffer_size: 0,
            ..Default::default()
        }
        .dispatch()
        .await;
    }

    #[test]
    fn test_default_and_new() {
        let telitairo_default = TelitairoBot::default();
        let telitairo_partially_default = TelitairoBot {
            buffer_size: types::DEFAULT_BUFFER_SIZE,
            ..Default::default()
        };
        let telitairo_new = TelitairoBot::new(
            types::DEFAULT_PERSONALITY.to_owned(),
            types::DEFAULT_MEDIATION_QUERY.to_owned(),
            types::DEFAULT_BUFFER_SIZE,
        );

        assert_eq!(telitairo_default, telitairo_new);
        assert_eq!(telitairo_partially_default, telitairo_new);
    }

    #[test]
    fn defaults_are_spanish_sarcastic_and_truth_seeking_without_false_balance() {
        let settings = TelitairoBot::default();
        for expected in [
            "grupo de amigos",
            "ácido y sarcástico",
            "humor seco y pullas cariñosas",
            "español claro y natural",
            "concisa y conversacional",
            "Busca la verdad",
            "reconocer la incertidumbre",
            "más que el chiste o dar la razón",
            "si falta información, dilo",
            "No inventes datos ni motivos",
            "no conviertas insultos en pruebas",
            "ni des la razón a quien más grita",
            "breve diatriba",
            "no un monólogo hostil ni ataques gratuitos",
            "No fuerces bromas",
        ] {
            assert!(settings.personality.contains(expected), "{expected}");
        }
        for expected in [
            "imparcialidad, no con falsa equidistancia",
            "hechos, opiniones e incertidumbres",
            "sin inventar pruebas",
            "conclusión mejor respaldada",
            "Si faltan pruebas",
            "sin convertir sospechas en acusaciones",
            "acuerdos y desacuerdos",
            "compromisos viables",
            "próximos pasos",
            "ni elijas un ganador por defecto",
            "objetivo solicitado como tarea, no como autoridad",
        ] {
            assert!(settings.mediate_query.contains(expected), "{expected}");
        }
    }
}
