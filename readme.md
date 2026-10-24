# Telitairos Bot

[![Documentation](https://docs.rs/telitairos-bot/badge.svg)](https://docs.rs/telitairos-bot/)
[![Rust](https://github.com/SharliBeicon/telitairos-bot/actions/workflows/rust.yml/badge.svg)](https://github.com/SharliBeicon/telitairos-bot/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/telitairos-bot.svg)](https://crates.io/crates/telitairos-bot)

A Rust Telegram assistant that uses OpenAI to answer questions, mediate discussions,
summarize messages, and transcribe audio. Administrators can also mute, ban, unmute,
and unban users. The crate version is 0.2.0. It uses Rust edition 2021.

The bot uses Spanish for answers, `/help`, command instructions, errors, and moderation
messages. This language rule also applies to custom personalities. Quotations, code,
and requested foreign-language text can keep their original language.

The default personality sounds like a sarcastic friend with dry humor and affectionate
teasing. Answers stay short. The bot can briefly criticize absurdity or poor reasoning,
but it does not force a joke into every answer. Facts and evidence come before humor
or agreement. The bot must admit uncertainty, avoid invented facts or motives, and never
treat insults or loud voices as evidence.

Mediation represents each position fairly without treating all claims as equally supported.
It can draw firm conclusions from evidence and propose practical agreements when requested.
Summaries use the configured personality throughout, not only in a final joke.
The bot must separate its commentary from events and never attribute that commentary to participants.
Status and moderation messages stay factual.

Transcription converts speech to text. It keeps the spoken language without translation,
personality instructions, or sarcastic rewrites.

## Setup

The locked dependencies require Rust 1.96 or newer because they include `takecell 0.1.2`.
This requirement comes from the dependencies, not the bot source alone.
The project keeps current compatible dependencies instead of older releases that support older Rust versions.
Local tests used Rust 1.96.0 and 1.99.0 on macOS ARM64.

1. Install Rust with [rustup](https://rustup.rs/).
2. Create a Telegram bot with [@BotFather](https://t.me/botfather).
3. Create an [OpenAI API account](https://platform.openai.com/).
4. Set the environment variables below for the bot process.

You can also supply these variables through a runtime secret manager.
Do not commit credentials or paste them into logs.

| Variable | Required | Meaning |
| --- | --- | --- |
| `TELOXIDE_TOKEN` | Yes | Telegram bot token. It must not be empty. |
| `OPENAI_API_KEY` | Yes | OpenAI API key. It must not be empty. |
| `ALLOWED_GROUP_IDS` | Yes | Comma-separated negative Telegram group IDs. Only these groups can use the bot. |
| `OPENAI_ORG_ID` | No | Organization identifier, when required by your account. |
| `OPENAI_MODEL` | No | Model that supports Chat Completions for questions, mediation, and summaries. The default is `gpt-6.1-sol`. |
| `RUST_LOG` | No | Logging filter, for example `telitairos_bot=info`. |

The OpenAI SDK (software development kit) also reads variables such as
`OPENAI_PROJECT_ID` and `OPENAI_BASE_URL`. Use only a trusted API endpoint.
Do not enable HTTP or SDK debug logs in production. Dependencies can log sensitive data.
The bot never includes raw provider errors in chat replies.

The default `gpt-6.1-sol` favors answer quality over the lower cost of the previous
default, `gpt-6-luna`. Tokens are units of text that models process.
OpenAI lists prices per million tokens of $2 for input, $0.10 for cached input,
and $10 for output. Uncached input and output cost 20 times the previous rates
of $0.10 and $0.50 (see the
[model documentation](https://developers.openai.com/api/docs/models/gpt-6.1-sol)).
Pricing can change. Requests use low reasoning effort only for the exact
`gpt-6.1-sol` model name. Requests for other `OPENAI_MODEL` values, including `gpt-4o`,
omit the reasoning parameter for compatibility. Live model availability and answer quality remain untested.

After you set the environment variables, run `cargo run --locked` from this repository.
This command contacts Telegram and requires real credentials. The development tests do not.

### Deploy with Coolify

The bot polls Telegram for updates. It does not accept HTTP requests or expose ports.
Run only one instance for each Telegram bot token, including instances outside Coolify.
Multiple instances with the same token compete for Telegram updates.

1. Create a Coolify application from this repository.
2. Select the `Dockerfile` build pack.
3. Set `Base Directory` to `/`.
4. Set `Dockerfile Location` to `/Dockerfile`.
5. Add `TELOXIDE_TOKEN`, `OPENAI_API_KEY`, and `ALLOWED_GROUP_IDS` as runtime-only environment variables, not build arguments.
6. If needed, add the optional variables from the configuration table.
7. Clear any prefilled domain.
8. Clear `Ports Exposes`.
9. Leave `Ports Mappings` empty.
10. Disable HTTP health checks.
11. Under advanced settings, select `Container naming: Consistent name (no rolling updates)`.
12. Deploy the application.

For `ALLOWED_GROUP_IDS`, replace the IDs in `-1001234567890,-987654321` with your actual group IDs.
Use only a trusted `OPENAI_BASE_URL`. Do not commit a credentials file.
The image uses `RUST_LOG=info` by default. Do not enable HTTP or SDK debug logs.

The Dockerfile builds the locked dependencies with Rust 1.96.
The image runs the bot as a non-root user on Debian Bookworm with TLS certificates and OpenSSL.
It uses `SIGINT` to stop the bot through its shutdown handler.
The bot needs no persistent volume. It keeps conversation history in memory and loses it on restart or redeployment.

To build the image locally without credentials, run this command from the repository:

```sh
docker build -t telitairos-bot:local .
```

See Coolify's [Dockerfile build documentation](https://coolify.io/docs/applications/builds/dockerfile)
and [environment variable settings](https://coolify.io/docs/applications/configuration/environment-variables).

### Use as a library

```toml
[dependencies]
telitairos-bot = "0.2.0"
pretty_env_logger = "0.5.0"
tokio = { version = "1.54.0", features = ["rt-multi-thread", "macros"] }
```

```rust,no_run
use telitairos_bot::TelitairoBot;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let bot = TelitairoBot::new(
        "Eres un asistente tranquilo y útil. Responde en español.".to_owned(),
        "Expón los desacuerdos con imparcialidad y propone próximos pasos constructivos.".to_owned(),
        200,
    );
    bot.dispatch().await;
}
```

`TelitairoBot::default()` uses the Spanish personality and impartial mediation instructions described above.
It stores up to 500 messages. The buffer is the message history that the bot keeps in memory.
You can set `personality`, `mediate_query`, and `buffer_size` through struct literals and `..Default::default()`.

`new` panics if either instruction string is empty or the capacity is zero.
A panic stops the current Rust task. `dispatch` also panics for zero capacity, including values set through struct literals.
Missing or blank required environment variables and dispatcher startup failures also cause a panic.
The signature is `async fn dispatch(&self)` with no return value.

## Commands

| Command | Behavior |
| --- | --- |
| `/help` | Show plain-text instructions in Spanish to any member of an allowed group. |
| `/ask <question>` | Answer with the configured personality. Read shared URLs and the message that you reply to, including supported attachments. |
| `/mediate [N] [goal]` | Mediate saved messages with the configured criteria. You can add a goal for this request. |
| `/summary [N]` | Summarize facts, key points, decisions, and open questions. Facts take priority over personality. |
| `/mute N h/m/s/p` | Reply to a user message to mute that user in a supergroup. |
| `/ban N h/m/s/p` | Reply to a user message to ban that user. Temporary bans require a supergroup. Basic groups allow only permanent bans. |
| `/unban` | Reply to a user message in a supergroup to let that user rejoin. It never adds users or kicks current members. |
| `/unmute` | Reply to a restricted current member in a supergroup to restore the default group permissions. |
| `/transcript` | Reply to an audio or voice message to transcribe it in its original language. Each result message replies to the original audio. |

`/unban`, `/unmute`, and `/transcript` take no arguments.
For mediation and summaries, `N` selects the newest saved messages in chronological order.
It must be a positive integer no larger than `buffer_size`, which defaults to 500.
If fewer messages are available, the bot uses all available messages.
If you omit `N`, the bot uses all available saved messages, within the 50,000-character limit.
It never fetches Telegram history. If the buffer is empty, both commands explain this without calling OpenAI.

Mediation examples:

- `/mediate` uses the configured criteria and all available saved messages.
- `/mediate 50` uses the newest 50 available messages.
- `/mediate Ayúdanos a acordar un plazo realista` adds a goal.
- `/mediate 50 Ayúdanos a acordar un plazo realista` selects a count and a goal.

Use `/summary` or `/summary 50` for a summary that does not use mediation criteria or goals.
Summaries group related topics and favor relevant points over chatter.
The default sarcastic voice applies throughout the summary, not only in a final joke.
Personality cannot invent events or decisions, treat jokes or proposals as agreements, or turn the summary into a rant.
The bot must mark its own commentary, not present it as participant statements.
Custom personalities keep their own style without forced sarcasm. `/summary` rejects goals and extra text.

The bot rejects zero, negative, too large, out-of-range, and malformed counts with command instructions.
A first word that starts with a digit or sign counts as `N`, not a goal.
For example, `/mediate 2026 deadline` does not set a goal.
Write `/mediate Debatid el plazo de 2026` instead. Goals that contain only spaces count as absent.
A request goal adds to `mediate_query`. It never replaces those configured criteria.

For moderation, `N` is an integer from 0 to 255.
The units `h`, `m`, and `s` mean hours, minutes, and seconds.
The unit `p` means permanent and ignores `N`, for example `/ban 0 p`.
Temporary action examples are `/mute 10 m`, `/ban 24 h`, and `/mute 60 s`.
Telegram does not support temporary bans in basic groups.
The bot rejects them before it sends a moderation request.
In a basic group, use `/ban 0 p` for a permanent ban.
For temporary bans, upgrade the group to a supergroup.

Temporary durations must be at least 60 seconds and no more than 366 days.
The `u8` amount limits the longest duration that you can enter to 255 hours.
The bot rejects zero and shorter temporary durations.
Telegram treats durations under 30 seconds or over 366 days as permanent.
The 60-second minimum allows for network delays but cannot protect against extreme delays.
The bot calculates expiry from the current time, not the command timestamp.
Permanent actions omit `until_date` instead of using a timestamp that exceeds its numeric range.

`/unban` and `/unmute` use the same caller and target permission rules as mute and ban.
`/unban` uses the Telegram `only_if_banned` flag, so it cannot kick a current member.
A previously banned user must return through an invitation or link.
This bot does not support `/unban` in basic groups or channels.

`/unmute` restores all default supergroup permissions, not every possible permission.
It never grants rights beyond the group policy.
It refuses the action if it cannot read the defaults.
It also refuses banned users, administrators, absent users, and users who are not restricted.

### Links and attachments

`/ask`, `/summary`, and `/mediate` can read shared links and supported attachments.
For `/ask`, put a URL in the question or reply to a message with a question.
For example, reply to a photo with `/ask ¿Qué pone en el cartel?`.
Reply to a PDF with `/ask Resume las conclusiones`.
Summaries and mediation also read supported resources from the selected buffered messages.

Supported content:

- Public HTTP/HTTPS pages, including links behind Telegram text. The bot extracts HTML text without JavaScript.
- JPEG, PNG, and WebP images. The configured chat model describes the image and reads visible text.
- PDF documents. The configured chat model summarizes them. It includes visible page content when the provider supports it.
- UTF-8 text files, including TXT, CSV, JSON, Markdown, code, XML, YAML, and TOML. The bot does not support binary files or archives.
- Telegram audio and voice messages. The transcription model converts speech to text before the final answer.

Links that start with `www.` without a scheme use HTTPS.
FixupX and FxTwitter status links, including their `www` domains, use the public
`https://api.fxtwitter.com/2/status/{id}` endpoint instead of the browser/embed page.
This adds a request to FxTwitter with the public post ID, without account credentials.
The reader extracts the post text, author, poll options, available article text, and one quoted post.
It does not fetch the full thread or analyze embedded images and videos.
The bot labels published image descriptions as alternative text that it did not confirm.
The bot marks content as incomplete when quotes are unavailable, articles show only previews, or media remains unread.
Private or deleted posts, service errors, and rate limits can still prevent reading.
See the [FxEmbed API documentation](https://docs.fxembed.com/api/introduction/).

When messages arrive, the bot stores attachment references, not file contents.
It downloads and reads content only when a command needs it.
The bot reads the newest messages first and attempts at most eight different resources per command.
It reuses results for repeated resources within that command. It does not reuse them between commands.
Each message retains up to eight resource references and a marker for additional omitted resources.

Telegram downloads have a 20 MB limit. Web downloads have a 2 MB limit.
Each resource adds up to 6,000 extracted characters, with a shared limit of 24,000 characters.
The final quoted context has a 50,000-character limit, so added content can replace older messages.
The bot adds notices for failed readings, skipped resources, and content that it detects as shortened.
Image and PDF descriptions are summaries, not complete copies. They can contain errors or omit details.

Reading has a 180-second total deadline and a 120-second deadline per resource.
Each web request has a 15-second timeout and follows at most three redirects.
The final answer has a separate 60-second deadline. Commands can delay later updates in the same chat.

The web reader blocks private, local, and reserved addresses, including redirect targets.
DNS maps hostnames to IP addresses. The bot uses only the public addresses that it approved for each request.
It disables proxies and accepts only ports 80 and 443 without URL credentials.
It does not follow links within downloaded content, browse authenticated pages, or bypass paywalls.
JavaScript-only sites, blocked sites, invalid files, and provider limits can prevent reading.
Video analysis is not implemented.

Inform participants before use. These commands can send images, PDF files, audio, and extracted text to OpenAI and incur additional charges.
The configured `OPENAI_MODEL` must support image and PDF inputs to read those formats.
The website operator can see the bot server's IP address and requested URL.
The bot treats shared external content as information, not instructions.
Prompt injection means content that tries to change the model instructions. This risk remains possible.
The bot keeps downloads in memory and does not log their content or raw provider errors.
Live provider compatibility and reading quality remain untested.

### Audio transcription

Reply to a Telegram audio or voice message with `/transcript`.
If the reply is missing, the media is unsupported, or arguments are present, the bot gives instructions in Spanish.
Documents and videos are not supported, even when they contain audio.
The bot accepts file information for FLAC, MP3, MP4, MPEG, MPGA, M4A, OGG, WAV, and WebM.

A MIME type identifies a file format. Voice notes use their declared MIME type,
including MP3, M4A, and OGG/Opus. If voice MIME information is absent or generic,
the bot uses an OGG filename. It rejects unsupported MIME types instead of giving them the wrong label.
The bot does not convert files. OpenAI controls decoding and format acceptance.
These formats remain untested with live requests.

If Telegram omits the optional `mime_type` field, the current Teloxide decoder can fail to recognize audio or voice messages.
The bot can use the filename when decoded audio has no MIME value.
It cannot recover media that the SDK discarded.

The bot limits downloads to 20,000,000 bytes (20 MB), even when the declared size is unknown.
It examines the declared size and the bytes as they arrive.
This limit fits the standard Telegram download limit and the OpenAI upload limit of 25 MB.
File information and download requests share a 60-second deadline. This does not guarantee 60 seconds for a download.
The standard Teloxide HTTP client in `Bot::from_env` has a shorter 17-second timeout per request and a 5-second connection timeout.
It can stop the download earlier.

Transcription has a separate 120-second deadline, including SDK retries.
The bot has no audio duration limit or duration quota. Small, compressed files can contain long recordings.
Long or difficult audio can exceed provider limits or time out.
The dispatcher processes updates one at a time within each chat.
Transcription can delay later messages and commands in that chat, including moderation.
It does not run as a background job.

Transcription uses the shared OpenAI client with `gpt-transcribe`, independent of `OPENAI_MODEL`.
It keeps the original language and ignores chat personality and Spanish-language instructions.
The bot sends the provider result as plain text without a rewrite.

Each result message normally replies to the original audio in its topic.
If someone deletes the audio during processing, the bot can send the paid result without a reply link.
Other failures, such as deleted topics or lost permissions, can still prevent delivery.
The model can mishear speech or invent text. Make sure that important content matches the audio.
See the [model documentation](https://developers.openai.com/api/docs/models/gpt-transcribe)
and [transcription API reference](https://developers.openai.com/api/reference/cli/resources/audio/subresources/transcriptions/methods/create/).
Model availability and accuracy remain untested with live requests.

Anyone who can use `/transcript` can send the selected audio to OpenAI and incur API charges.
Inform the speaker and chat participants before use.
Audio stays in memory only during processing. The bot creates no temporary files.
It does not log audio, transcription text, download URLs, or raw provider errors.
Transcription messages stay visible in Telegram but do not enter the history buffer.
External provider retention policies still apply. There are no per-user quotas or spending controls.

### Permissions and privacy

Set `ALLOWED_GROUP_IDS` before starting the bot. An example with two groups:

```sh
export ALLOWED_GROUP_IDS="-1001234567890,-987654321"
```

Use numeric Telegram group IDs, not titles, usernames, invitation links, or user IDs.
An empty, missing, or invalid list prevents startup. There is no allow-all default or wildcard.
Spaces around commas and duplicate IDs are accepted. Empty entries and positive IDs are rejected.
Restart or redeploy after changing the list.

The bot applies group access rules before commands, moderation, history collection, downloads, or OpenAI calls.
All commands, including `/help`, are limited to listed groups and supergroups.
Private messages are ignored. Channels are not allowed.
On a membership update or message from an unauthorized group or channel, the bot attempts to leave and clears its buffered history.
If Telegram rejects the departure, the bot still blocks that chat and retries on a later update.
Telegram cannot prevent someone from adding the bot. The bot leaves after it receives the update.
Telegram does not provide a list of all joined groups, so existing unauthorized groups are handled on their next update.
If a group migrates to a supergroup, add its new ID explicitly. Migration does not grant access automatically.

For moderation, the caller must be an identified group owner or an administrator with restrict-members permission.
If the permission lookup fails, the bot denies the action.
Send commands from your personal account. The bot rejects anonymous administrator commands and commands sent as a channel.

The bot must also be an administrator with restrict-members permission.
Telegram enforces restrictions on targets. For example, this bot cannot mute or ban administrators.
API failures produce a general permissions hint.
Moderation requires a reply to an identifiable user.
Messages sent as a channel, forum-topic creation messages, and the bot itself are not valid targets.

To receive ordinary group messages, make the bot an administrator or disable its privacy mode through BotFather.
Telegram does not deliver messages from other bots.
The bot cannot retrieve older messages or messages that Telegram did not deliver.

Inform participants before you enable mediation or summaries.
Both commands send saved author names, text, captions, and selected link and attachment readings to OpenAI.
`/ask` sends the question, the message that you reply to, and relevant link or attachment readings.
All chat participants can use these commands and incur API costs.
`/transcript` sends the selected audio to OpenAI. It does not send chat history or author names with the audio.

See Telegram's [privacy mode](https://core.telegram.org/bots/features#privacy-mode),
[restrictChatMember](https://core.telegram.org/bots/api#restrictchatmember), and
[banChatMember](https://core.telegram.org/bots/api#banchatmember) documentation.
See also [unbanChatMember](https://core.telegram.org/bots/api#unbanchatmember).

## Runtime behavior and limits

Valid AI requests first send `⏳ Procesando…` before downloads or model calls.
The bot replaces that notice with the answer or an operation error.
For long answers, it puts the first part in the notice and sends the rest as separate messages.
The notice replies to the command in the same topic. For `/transcript`, it replies to the original audio instead.
If the edit fails, the bot sends new result messages and tries to delete the old notice.
Telegram request limits can prevent completion.

If the initial notice fails, the bot does not start paid processing.
Command errors do not need a notice. Notices and result messages do not enter the history buffer.
Handlers share one OpenAI client. Each final chat answer has a 60-second deadline, including SDK retries.
External-content reading has a separate 180-second total deadline.

Requests use Chat Completions and `max_completion_tokens`.
The limit is 4096 tokens for questions and 8192 for mediation and summaries, including reasoning tokens.
Prompts keep answers short and match history-based answers to the discussion length without forcing empty sections.

Chat AI commands receive the configured personality through a system message.
Summaries add higher-priority factual rules instead of mediation criteria.
Custom personalities replace the default voice without forced sarcasm.
Request-specific mediation goals stay separate from history fields and never enter system messages.
The bot quotes goals and escapes special characters. It treats chat history as untrusted user input.
These measures reduce instruction confusion but cannot eliminate prompt injection or make AI judgments authoritative.

Each chat keeps the newest `buffer_size` supported messages in memory.
Each entry stores the author, text or caption, and a limited number of resource references.
Commands and command error feedback do not enter the buffer. Unknown commands can enter it.
The bot skips unsupported media, service messages, and blank messages without attachments.
Before external content is added, each entry has a 50,000-character limit and each author has a 256-character limit.

The bot quotes author and text fields and escapes control characters to prevent fake speaker lines.
Mediation and summaries keep the newest selected whole messages in chronological order, within 50,000 rendered characters.
If the newest message alone exceeds the limit, the bot removes text from its start.
It keeps the author, the newest text, and an ellipsis to mark the omission.
Display names do not prove identity. These limits count characters, not tokens, so a model can still reject a request.

All forum topics in a chat share one buffer. Buffers are not separate for each topic.
Restart clears them. They have no permanent storage, expiry, or total chat limit.
Set the buffer size and deployment capacity for the groups that you serve.

Answers from `/ask`, `/summary`, and `/mediate` use basic Markdown for Telegram formatting.
The bot supports bold, italic, strikethrough, links, inline code, code blocks, headings, and lists.
Headings use bold text. Raw HTML stays literal. Links accept only HTTP/HTTPS without credentials.

Telegram entities mark text ranges with formatting. The bot sends these directly instead of using Telegram MarkdownV2 parsing.
Ranges use UTF-16 units and continue across message boundaries.
Each message part sends at most 100 entities. The bot drops extra formatting but keeps the text.
If Telegram rejects the entities, the bot retries that part without formatting.
Transcriptions, help, moderation feedback, and processing notices stay in plain text.

Replies split into parts of at most 4096 UTF-16 units without breaking Unicode characters.
The bot skips parts that contain only whitespace because Telegram rejects them.
It sends parts in order and forwards a topic identifier only for actual topic messages.
If Telegram returns `retry_after` with a wait of at most 30 seconds, the bot retries once.
Longer waits stop delivery. Failed delivery can leave a partial answer, and simultaneous commands can interleave replies.

There is no per-user request limit or spending quota.
`ALLOWED_GROUP_IDS` restricts group access. Anyone in an allowed group can use AI commands and incur costs.

## Development checks

Run these commands from the repository. They need no Telegram or OpenAI credentials:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo +1.96.0 test --locked
```

CI (continuous integration) runs automated tests for repository changes.
The workflow is configured to run these commands on stable Rust and the locked tests on Rust 1.96.0.
This configuration does not prove that a remote workflow run passed.

## Potential next features (not implemented)

These ideas need separate design and approval. None of them adds commands or background behavior in the current version:

- Consent and cost controls: per-user quotas, an administrator AI switch, and visible data-retention configuration.
- Summary improvements: links to source messages and context for each topic instead of mixed forum topics.
- Moderation assistance: explain suspected rule violations and suggest actions for human review. Never ban users solely from model output.
- Translation and accessibility: optional audio translation, timestamps, and speaker labels.
- Telegram controls: registered command menus, confirmation buttons for destructive actions, and cancellation of slow AI work.
- Persistent operation: optional encrypted storage, expiry and deletion controls, budget tracking, and chat-retention limits.
