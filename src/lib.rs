//! # Telitairos Bot 0.2.0
//!
//! A Telegram assistant with `/ask <question>`, `/mediate [N] [goal]`,
//! `/summary [N]`, and `/help` for members of allowed groups. Mediation accepts a count, a goal, both,
//! or neither; for example `/mediate 50 Ayúdanos a acordar un plazo realista`.
//! Summary accepts only an optional count and faithfully covers facts, key points,
//! decisions, and open questions, with factual constraints taking priority over personality.
//! Both default to all available buffered history.
//! Valid AI requests send a processing notice before downloads or model calls,
//! then edit it with the result or operation error. Long results continue in new
//! messages. Notices reply to the command, or to the original audio for `/transcript`.
//! Failed edits fall back to new messages and attempted notice deletion, except
//! when Telegram rate limits stop delivery. Failed notices stop paid processing.
//! Chat AI commands read shared public URLs, images, PDF and UTF-8 text files,
//! plus Telegram audio through transcription. `/ask` also reads the replied-to message.
//! Resource references are buffered without downloads. Commands attempt up to eight
//! distinct resources on demand, with a 180-second reading deadline before the answer.
//! Downloads are limited to 20 MB from Telegram and 2 MB from the web. External text
//! is capped at 6000 characters per resource and 24,000 per command before the final
//! 50,000-character context cap. Failed readings and detected omissions produce notices.
//! External content is untrusted. Public URL fetching blocks private/reserved networks,
//! validates redirects and pins DNS results. It does not run JavaScript or analyze video.
//! `N` selects the newest available messages and must be an integer from 1 to
//! `buffer_size`; fewer available messages means using those available. Invalid
//! counts (including overflow, negative, or malformed numeric-looking first tokens)
//! are rejected. A numeric-looking first token is reserved for the count; start
//! numeric mediation goals with words. Goals supplement configured mediation criteria.
//! Administrators with restrict-members permission can reply to a user's message
//! with `/mute N h/m/s/p` (supergroups only) or `/ban N h/m/s/p` (groups and
//! supergroups; finite bans require a supergroup). `N` is 0–255; `p` means permanent, ignoring `N`. Finite durations
//! must be at least 60 seconds, leaving a margin above Telegram's 30-second
//! permanent-action threshold. Anonymous administrator commands are rejected.
//! The bot also needs administrator restrict-members permission for moderation.
//! `/unban` and `/unmute` take no arguments and require the same administrator and
//! reply-target checks, in supergroups only. `/unban` lets a banned user rejoin via
//! invitation or link; it never kicks an existing member or automatically adds anyone.
//! `/unmute` restores a restricted current member's permissions to the supergroup
//! defaults, not unrestricted permissions. Missing defaults deny the action.
//!
//! Reply to an audio or voice message with `/transcript` (no arguments) to send its
//! audio to OpenAI's dedicated `gpt-transcribe` model, independently of `OPENAI_MODEL`.
//! The returned text replies to the original audio, in its topic when applicable,
//! including every chunk of a long transcription. If the original message was deleted,
//! delivery is allowed without a reply anchor. Speech retains its original language;
//! personality and Spanish chat instructions do not apply. This is transcription, not
//! translation or sarcastic paraphrasing, but model errors remain possible.
//! Only Telegram audio and voice messages are supported, not documents or video.
//! Supported formats are FLAC, MP3, MP4, MPEG, MPGA, M4A, OGG (including voice Opus),
//! WAV and WebM, subject to provider support; there is no live format validation.
//! Voice MIME types select the format too (including MP3/M4A); only absent or generic
//! voice MIME metadata falls back to OGG. Unsupported MIME types are rejected.
//! Downloads are limited to 20,000,000 bytes, checked against metadata and the stream,
//! with a 60-second outer deadline. The default Telegram HTTP client's shorter
//! 17-second per-request timeout (5-second connection timeout) can abort earlier.
//! Audio stays in memory, with no temporary files or logs; transcription has a
//! separate 120-second deadline including SDK retries. There is no audio duration
//! cap or duration quota. Updates in each chat are processed sequentially, so
//! transcription can delay later commands and messages in that chat, including moderation.
//!
//! ## Configuration and privacy
//!
//! Required process environment: `TELOXIDE_TOKEN`, `OPENAI_API_KEY`, and `ALLOWED_GROUP_IDS`.
//! `ALLOWED_GROUP_IDS` is a nonempty comma-separated list of negative group IDs.
//! Access is checked before commands and history collection. Only listed groups and
//! supergroups are allowed. Private messages are ignored. On an unauthorized group's
//! membership update or message, the bot attempts to leave and clears its buffered history.
//! Channels are also rejected. A failed departure never grants access. Missing or invalid
//! configuration prevents startup. Existing groups are checked when their next update arrives.
//! Restart after changing the list. Migrated supergroups need their new ID added explicitly.
//! Bot replies and default prompts are in Spanish, including help and error feedback.
//! A common language instruction also applies to custom personalities.
//! `OPENAI_ORG_ID` is optional. `OPENAI_MODEL` defaults to `gpt-6.1-sol` and must support
//! Chat Completions, with image/PDF inputs for those resources. Final chat answers have a
//! 60-second deadline after resource reading. Never log credentials.
//! Low reasoning effort is sent only for the exact default model; arbitrary overrides
//! omit it. Completion budgets are 4096 tokens for questions and 8192 for history commands,
//! including reasoning tokens. The default personality uses dry sarcasm and affectionate
//! friend-group banter, allowing a brief rant when warranted, but puts truth, evidence,
//! and uncertainty ahead of jokes or validation. Mediation avoids false balance and
//! can draw sharp evidence-backed conclusions or help find actionable agreements.
//! Summaries use the configured voice throughout, including integrated dry wit and
//! affectionate sarcasm with the default personality, rather than only a closing joke;
//! they never invent events or attribute the bot's commentary to participants and remain
//! independent of mediation criteria and invocation-specific goals.
//!
//! Chat AI commands can send images, PDF files, audio and extracted website/file text to
//! OpenAI. Mediation and summaries also send author names. Repeated commands reread resources
//! and can incur additional charges. Website operators see requests from the bot server.
//! Inform participants
//! first. `/transcript` sends the replied-to audio to OpenAI and can incur costs;
//! inform the speaker and participants before using it. To receive ordinary group messages, make the bot an administrator or
//! disable Telegram privacy mode. Telegram does not deliver other bots' messages.
//! The bot only knows messages delivered while it is running; it never fetches Telegram history.
//!
//! History is memory-only and per chat (shared across topics), retaining the newest
//! `buffer_size` supported messages not handled as commands. Restarting clears it. Entries and
//! history transcripts are capped at 50,000 Unicode characters. Transcript fields
//! are quoted/escaped and retain newest whole messages; an oversized newest
//! message keeps its author and newest text. There is no expiry, global chat-count bound, or
//! spending quota. All members of allowed groups can invoke AI commands; monitor costs.
//! Chat AI answers render basic Markdown as Telegram entities, including bold, italic,
//! links and code, with formatting ranges adjusted for each chunk. Raw HTML remains
//! literal. Transcriptions and mechanical feedback remain plain text. Replies are
//! split at 4096 UTF-16 units (whitespace-only chunks skipped),
//! and sent in the original topic only for actual topic messages. Rate-limit retries
//! wait at most 30 seconds.
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

/// Defines the bot behavior
#[derive(Clone, Debug, PartialEq)]
pub struct TelitairoBot {
    /// Configurable voice for chat AI commands; summaries prioritize facts, transcription ignores it.
    ///
    /// # Example
    /// ```bash
    /// "Eres un asistente cercano, útil y respetuoso. Responde en español."
    /// ```
    pub personality: String,
    /// String to define the bot action when `/mediate` command is sent. descriptive short prompt.
    ///
    /// # Example
    /// ```bash
    /// "Facilita un acuerdo neutral y propone próximos pasos concretos sin inventar pruebas."
    /// ```
    pub mediate_query: String,
    /// Buffer capacity, size of the context for `/mediate` and `/summary`.
    ///
    /// `new` and `dispatch` panic if `buffer_size` is zero.
    pub buffer_size: usize,
}

impl TelitairoBot {
    /// Creates a new bot with the selected parameters
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

    /// Listen for incoming messages and commands.
    ///
    /// # Panics
    /// Panics if `buffer_size` is zero, required environment variables are missing
    /// or empty, or the Telegram dispatcher cannot initialize.
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

/// Default attributes for a sarcastic, truth-seeking friend-group assistant.
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
        // build() type-checks every handler's injected dependencies, without dispatching.
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
