# Telitairos Bot

[![Documentation](https://docs.rs/telitairos-bot/badge.svg)](https://docs.rs/telitairos-bot/)
[![Rust](https://github.com/SharliBeicon/telitairos-bot/actions/workflows/rust.yml/badge.svg)](https://github.com/SharliBeicon/telitairos-bot/actions/workflows/rust.yml)
[![Crates.io](https://img.shields.io/crates/v/telitairos-bot.svg)](https://crates.io/crates/telitairos-bot)

A Rust Telegram assistant with OpenAI-powered questions, conversation mediation, summaries,
and audio transcription, plus administrator-only mute, ban, unmute, and unban commands. The crate remains version **0.2.0**
and uses Rust edition 2021.

The bot is Spanish-first: default prompts, generated answers, command help, usage
guidance, and error/moderation feedback are in Spanish. A shared Spanish-language
instruction applies even to custom personalities; quotations, code, and requested
foreign-language text can retain their original language. The default assistant
sounds like a friend with dry, pointed wit and affectionate banter: somewhat acidic
and sarcastic, concise, and willing to rant briefly at absurdity or bad reasoning
without forcing a joke into every reply. Truth, evidence, and acknowledged uncertainty
come before comedy or validation; it must not invent facts or motives, treat insults
as evidence, or side with whoever shouts loudest. Mediation fairly represents positions
without false balance, allowing sharp evidence-backed conclusions and actionable
agreements when requested. Summaries use the configured voice throughout, with dry wit
and affectionate sarcasm woven into the narration when using the default personality.
Facts still come first: the bot's commentary must be distinguishable from what happened
and never attributed to participants. Mechanical feedback stays factual.
Audio transcription is the exception to Spanish-first output: it preserves the
spoken language without translation, personality instructions, or sarcastic paraphrasing.

## Setup

1. Install Rust with [rustup](https://rustup.rs/). The current locked dependency
   graph requires **Rust 1.96 or newer**, driven by the refreshed transitive
   `takecell 0.1.2` dependency. This is a requirement of the current dependency
   graph, not a fundamental requirement of the bot's own source. We retain the
   latest compatible dependencies rather than pinning an older transitive release
   solely to advertise a lower minimum. It has been tested locally on Rust 1.96.0
   and 1.99.0 on macOS ARM64.
2. Create a Telegram bot with [@BotFather](https://t.me/botfather).
3. Create an [OpenAI API account](https://platform.openai.com/).
4. Supply configuration through your process environment or a runtime secret
   manager. Never commit credentials or paste them into logs.

| Variable | Required | Meaning |
| --- | --- | --- |
| `TELOXIDE_TOKEN` | Yes | Telegram bot token; must be nonempty. |
| `OPENAI_API_KEY` | Yes | OpenAI API key; must be nonempty. |
| `ALLOWED_GROUP_IDS` | Yes | Comma-separated negative Telegram group IDs. Only these groups can use the bot. |
| `OPENAI_ORG_ID` | No | Organization identifier, when required by your account. |
| `OPENAI_MODEL` | No | Chat Completions-compatible model for questions, mediation, and summaries; defaults to `gpt-6.1-sol`. |
| `RUST_LOG` | No | Logging filter, for example `telitairos_bot=info`. |

The OpenAI SDK also honors its own configuration variables such as
`OPENAI_PROJECT_ID` and `OPENAI_BASE_URL`. Only use a trusted API endpoint.
Keep HTTP/SDK debug logging disabled in production because dependencies can log
sensitive data. This bot never includes raw provider errors in chat replies.

The default `gpt-6.1-sol` prioritizes response quality over the previous low-cost
`gpt-6-luna` default. OpenAI lists standard prices per million tokens of $2 input,
$0.10 cached input, and $10 output: uncached input and output are 20 times the
previous model's $0.10/$0.50 rates (see the
[model documentation](https://developers.openai.com/api/docs/models/gpt-6.1-sol)).
Pricing can change. Requests use low reasoning effort only for the exact
`gpt-6.1-sol` model name. Arbitrary `OPENAI_MODEL` overrides, including `gpt-4o`,
omit the reasoning parameter for compatibility. No live availability or response
quality validation is implied.

From this repository, `cargo run --locked` starts the default bot once the
environment is configured. This command contacts Telegram and requires real
credentials; the tests below do not.

### Deploy with Coolify

1. Create a Coolify application from this repository and select the **Dockerfile**
   build pack. Set **Base Directory** to `/` and **Dockerfile Location** to
   `/Dockerfile`.
2. Add `TELOXIDE_TOKEN`, `OPENAI_API_KEY`, and `ALLOWED_GROUP_IDS` as **runtime-only** environment
   variables, not build arguments. Optionally set `OPENAI_MODEL` (default:
   `gpt-6.1-sol`), `OPENAI_ORG_ID`, `OPENAI_PROJECT_ID`, or a trusted
   `OPENAI_BASE_URL`. Do not commit a credentials file. The image defaults to
   `RUST_LOG=info`; avoid HTTP/SDK debug logging.
   For example, set `ALLOWED_GROUP_IDS=-1001234567890,-987654321` with your actual group IDs.
3. Clear any prefilled domain and **Ports Exposes**, leave **Ports Mappings**
   empty, and disable HTTP health checks. This is an outbound Telegram
   long-polling worker with no HTTP listener or exposed ports.
4. Under advanced settings, select **Container naming: Consistent name (no rolling
   updates)**. Run only **one instance per Telegram bot token**, including outside
   Coolify, so deployments do not overlap Telegram pollers. Deploy the application.

The multi-stage image builds the locked dependencies with Rust 1.96 and runs the
binary as a non-root user on Debian Bookworm with TLS certificates and OpenSSL.
It uses `SIGINT` for the bot's graceful shutdown handler. No persistent volume is
needed: conversation history is in memory and is lost on restart or redeployment.

To check the image build locally without credentials:

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

`TelitairoBot::default()` uses the Spanish sarcastic-but-factual assistant and impartial mediation prompts,
with a 500-message capacity. The public fields `personality`, `mediate_query`, and
`buffer_size` remain configurable with struct literals and `..Default::default()`.
`new` panics for empty personality/criteria or zero capacity. `dispatch` also
rejects zero capacity, including when constructed with a struct literal, and
panics for missing/blank required environment variables or dispatcher startup
failure. Its signature remains `async fn dispatch(&self)` with no return value.

## Commands

| Command | Behavior |
| --- | --- |
| `/help` | Show plain-text command help to any member of an allowed group. |
| `/ask <question>` | Answer using the configured personality. Read shared URLs and the replied-to message, including supported attachments. |
| `/mediate [N] [goal]` | Mediate buffered conversation using configured criteria, optionally supplemented by an invocation-specific goal. |
| `/summary [N]` | Faithfully summarize facts, key points, decisions, and open questions; factual constraints override personality. |
| `/mute N h/m/s/p` | Reply to a user's message to mute that user in a **supergroup**. |
| `/ban N h/m/s/p` | Reply to a user's message to ban that user. Finite bans require a supergroup; basic groups allow only permanent bans. |
| `/unban` | Reply to a user's message in a **supergroup** to let them rejoin; does not add them automatically or kick current members. No arguments. |
| `/unmute` | Reply to a restricted current member's message in a **supergroup** to restore the group's default permissions. No arguments. |
| `/transcript` | Reply to an audio or voice message to transcribe it in its original language; every result chunk replies to the **original audio**, not the command. No arguments. |

For mediation and summaries, omit `N` to use all available per-chat buffered history,
within `buffer_size` (500 by default) and the 50,000-character transcript cap.
`N` must be a positive integer no larger than configured `buffer_size`. It selects
the newest N available buffered messages, in chronological order; if fewer are
available, all available messages are used. No Telegram history is fetched.
Both commands explain when the buffer is empty without calling OpenAI.

Mediation examples:

- `/mediate` preserves the configured default behavior.
- `/mediate 50` uses the newest 50 available messages.
- `/mediate Ayúdanos a acordar un plazo realista` adds a goal.
- `/mediate 50 Ayúdanos a acordar un plazo realista` selects a count and a goal.

Use `/summary` or `/summary 50` for a faithful summary, independent of the configured
mediation criteria and invocation-specific goals. The default acidic, sarcastic voice
applies throughout the summary, not just in a closing joke. Summaries group related
topics and prioritize relevant points over chatter. Personality cannot invent events
or decisions, turn jokes or proposals into agreements, or turn the summary into a rant.
The bot's quips must remain recognizable as its own commentary, not participants' words.
Custom personalities retain their own style without forced sarcasm.
Summary rejects goals and trailing text.
Zero, negative, overflowing, out-of-range, and malformed counts are rejected with
usage guidance. A first token beginning with a digit or sign is reserved for the
count, so `/mediate 2026 deadline` is not interpreted as a goal. Write
`/mediate Debatid el plazo de 2026` instead. Whitespace-only goals are absent.
Invocation-specific goals supplement, never replace, the configured `mediate_query`.

For moderation, `N` is an integer from 0 to 255; `h`, `m`, and `s` mean hours,
minutes, and seconds. `p` means permanent and ignores `N`, for example `/ban 0 p`.
Examples of finite actions: `/mute 10 m`, `/ban 24 h`, `/mute 60 s`.
Telegram does not support ban expiry in basic groups, so finite basic-group bans
are rejected before any moderation request. Use `/ban 0 p` for a permanent ban
there, or upgrade the group to a supergroup for finite bans.

Finite durations must be **at least 60 seconds** and no more than 366 days;
the `u8` amount currently limits the maximum expressible finite duration to
255 hours. Zero and shorter finite spans are rejected. Telegram treats spans
under 30 seconds or over 366 days as permanent; the 60-second minimum leaves
a network-delay margin, not a guarantee against extreme transport delays.
Finite expiry is computed from the current time, not the command timestamp.
Permanent actions omit `until_date` instead of using an overflowing timestamp.

The reversal commands use the same caller-permission and reply-target checks as
mute and ban. `/unban` uses Telegram's `only_if_banned` flag, so it cannot kick a
current member; a previously banned user must return using an invitation or link.
It is not supported here in basic groups or channels. `/unmute` restores **all
default supergroup permissions**, not every possible permission: it does not grant
rights beyond the group policy. It refuses to act if defaults cannot be retrieved
or if the target is banned, an administrator, absent, or not restricted.

### Links and attachments

`/ask`, `/summary`, and `/mediate` can read shared links and supported attachments.
For `/ask`, put a URL in the question or reply to a message with a question.
For example, reply to a photo with `/ask ¿Qué pone en el cartel?`.
Reply to a PDF with `/ask Resume las conclusiones`.
Summaries and mediation also read supported resources from the selected buffered messages.

Supported content:

- Public HTTP/HTTPS pages, including links hidden behind Telegram text. The reader extracts HTML text without running JavaScript.
- JPEG, PNG, and WebP images. The configured chat model describes the image and reads visible text.
- PDF documents. The configured chat model extracts a summary, including visible page content when supported by the provider.
- UTF-8 text files, including TXT, CSV, JSON, Markdown, code, XML, YAML, and TOML. Binary files and archives are not supported.
- Telegram audio and voice messages. The dedicated transcription model converts speech to text before the final answer.

Links that start with `www.` without a scheme use HTTPS.
FixupX and FxTwitter status links, including their `www` domains, use the public
`https://api.fxtwitter.com/2/status/{id}` endpoint instead of the browser/embed page.
This adds a request to FxTwitter with the public post ID, without account credentials.
The reader extracts the post text, author, poll options, available article text, and one quoted post.
It does not fetch the full thread or analyze embedded images and videos.
Published image descriptions are labeled as unverified alternative text.
Unavailable quotes, article previews, and unread media produce partial-content markers.
Private or deleted posts, service errors, and rate limits can still prevent reading.
See the [FxEmbed API documentation](https://docs.fxembed.com/api/introduction/).

The bot stores attachment references, not their bytes, when messages arrive.
It downloads and reads content only when a command needs it.
Reading uses the newest messages first and attempts at most eight distinct resources per command.
Repeated resources share their reading result within that command. There is no cache between commands.
Each message retains up to eight resource references and a marker for additional omitted resources.

Telegram downloads have a 20 MB limit. Web downloads have a 2 MB limit.
Each reading contributes up to 6,000 characters, within a shared 24,000-character extracted-text budget.
The final quoted context still has a 50,000-character limit, so added content can displace older messages.
The bot appends notices for failed readings, omitted resources, and detected truncation.
Image and PDF descriptions are summaries, not complete copies. Recognition errors and omitted details remain possible.

Reading has a 180-second total deadline and a 120-second deadline per resource.
Each web request has a 15-second timeout and follows at most three redirects.
The final answer has a separate 60-second deadline. Commands can delay later updates in the same chat.

The web reader blocks private, local, and reserved addresses, including redirect targets.
It pins validated DNS addresses, disables proxies, and accepts only ports 80 and 443 without URL credentials.
It does not follow links within downloaded content, browse authenticated pages, or bypass paywalls.
JavaScript-only sites, blocked sites, invalid files, and provider limits can prevent reading.
Video analysis is not implemented.

Inform participants before use. These commands can send images, PDF files, audio, and extracted text to OpenAI and incur additional charges.
The configured `OPENAI_MODEL` must support image and PDF inputs to read those formats.
The website operator can see the bot server's IP address and requested URL.
Shared external content is untrusted information, not instructions, but prompt injection remains possible.
The bot keeps downloads in memory and does not log their content or raw provider errors.
Live provider compatibility and reading quality have not been tested.

### Audio transcription

Reply to a Telegram audio or voice message with `/transcript`. Missing replies,
other media, and extra arguments produce Spanish usage guidance. Documents and
videos are not supported, even when they contain audio. The bot accepts FLAC,
MP3, MP4, MPEG, MPGA, M4A, OGG, WAV and WebM metadata. Voice notes use their declared
MIME type too, including MP3 and M4A as well as OGG/Opus. Only absent or generic
voice MIME metadata falls back to an OGG filename; clearly unsupported MIME types
are rejected instead of mislabeled. It does not convert files. Actual decoding and
format acceptance depend on OpenAI; these formats have not been tested live.
The current Teloxide decoder may fail to recognize audio/voice media when Telegram
entirely omits its optional `mime_type` field. Filename fallback works when the
decoded audio has no MIME value, but cannot recover media discarded by the SDK.

The bot checks declared file sizes and incrementally bounds downloads to
**20,000,000 bytes (20 MB)**, including when the declared size is unknown. This
conservative limit fits Telegram's standard download limit and OpenAI's 25 MB
upload limit. Getting file metadata and downloading together have a **60-second
outer deadline**, not a guaranteed download window: the standard Teloxide HTTP
client used by `Bot::from_env` has a shorter **17-second timeout per request**
(and a 5-second connection timeout), so it can abort earlier. Transcription has a
separate 120-second deadline, including SDK retries. There is no additional audio
duration cap or duration quota: small, highly compressed files can contain long
recordings. Long or difficult audio can still exceed provider limits or time out.
The dispatcher processes updates sequentially within each chat, so transcription
can delay subsequent messages and commands in that chat, including moderation.
It does not run as a background job.

Transcription uses the shared OpenAI client with the dedicated **`gpt-transcribe`**
model, independent of `OPENAI_MODEL`. It requests original-language transcription,
not translation, and applies none of the chat personality or Spanish-language
instructions. The provider's text is delivered as plain text without rewriting.
Every chunk normally replies to the original audio in its topic. If the audio
message was deleted during processing, delivery is allowed without a reply anchor
instead of discarding the paid result. Other delivery failures (such as a deleted
topic or lost permissions) can still prevent delivery.
Recognition mistakes or hallucinations remain possible, so verify important content.
See the [model documentation](https://developers.openai.com/api/docs/models/gpt-transcribe)
and [transcription API reference](https://developers.openai.com/api/reference/cli/resources/audio/subresources/transcriptions/methods/create/).
Model availability and accuracy have not been validated with live requests.

**Privacy and cost:** anyone able to invoke `/transcript` can send the replied-to
audio to OpenAI and incur API charges. Inform the speaker and chat participants
before use. Audio bytes stay in memory only during processing, with no temporary
files; the bot does not log audio, transcription text, download URLs, or raw
provider errors. Generated transcription messages are not added to its history
buffer, but remain visible in Telegram. External provider retention policies still
apply. There are no per-user quotas or spending controls.

### Permissions and privacy

Set `ALLOWED_GROUP_IDS` before starting the bot. An example with two groups:

```sh
export ALLOWED_GROUP_IDS="-1001234567890,-987654321"
```

Use numeric Telegram group IDs, not titles, usernames, invitation links, or user IDs.
An empty, missing, or invalid list prevents startup. There is no allow-all default or wildcard.
Spaces around commas and duplicate IDs are accepted. Empty entries and positive IDs are rejected.
Restart or redeploy after changing the list.

The bot checks group access before commands, moderation, history collection, downloads, or OpenAI calls.
All commands, including `/help`, are limited to listed groups and supergroups.
Private messages are ignored. Channels are not allowed.
On a membership update or message from an unauthorized group or channel, the bot attempts to leave and clears its buffered history.
If Telegram rejects the departure, the bot still blocks that chat and retries on a later update.
Telegram cannot prevent someone from adding the bot. The bot leaves after it receives the update.
Telegram does not provide a list of all joined groups, so existing unauthorized groups are handled on their next update.
If a group migrates to a supergroup, add its new ID explicitly. Migration does not grant access automatically.

- The caller must be an identified group owner or an administrator with
  restrict-members permission. Permission lookup failures deny the action.
  Anonymous administrator/channel-sent commands are rejected; send as your
  personal account instead.
- The bot must itself be an administrator with restrict-members permission.
  Telegram enforces target restrictions (for example, administrators cannot be
  muted/banned by this bot). API failures produce a generic permissions hint.
- Moderation requires a reply to an identifiable user. Channel-sent messages,
  forum-topic creation messages, and the bot itself are not valid targets.
- To receive ordinary group conversation for mediation and summaries, make the bot an
  administrator or disable its privacy mode through BotFather. Telegram does
  not deliver other bots' messages to it. It cannot retrieve messages sent
  before it was running or otherwise not delivered to it.
- Inform participants before enabling mediation or summaries: both commands send buffered
  author names, text/captions, and readings of selected links and attachments to OpenAI.
  `/ask` sends the question, replied-to context, and relevant link or attachment readings. These
  commands remain available to all chat participants and can incur API costs.
- `/transcript` sends the replied-to audio to OpenAI, as described above. It does
  not send a chat-history transcript or author names along with the audio.

See Telegram's [privacy mode](https://core.telegram.org/bots/features#privacy-mode),
[restrictChatMember](https://core.telegram.org/bots/api#restrictchatmember), and
[banChatMember](https://core.telegram.org/bots/api#banchatmember) documentation.
See also [unbanChatMember](https://core.telegram.org/bots/api#unbanchatmember).

## Runtime behavior and limits

- Valid AI requests first send `⏳ Procesando…` before downloads or model calls.
  The bot edits that message with the answer or an operation error.
  For long answers, it edits the notice with the first chunk and sends the remaining chunks separately.
  The notice replies to the command in the same topic. For `/transcript`, it replies to the original audio instead.
  If editing fails, the bot sends new result messages and attempts to delete the stale notice.
  Telegram rate limits can prevent completion. If the initial notice cannot be sent, the bot does not start paid processing.
  Usage errors do not need a notice. Notices and result messages are not added to the history buffer.
- One reusable OpenAI client is shared by handlers. Each final chat answer has a
  60-second deadline, including SDK retries. External-content reading has a separate
  180-second total deadline. Requests use Chat Completions and
  `max_completion_tokens` (4096 for questions, 8192 for mediation and summaries,
  including reasoning tokens). Prompts keep visible responses concise and adapt
  history-based answers to the discussion length without forcing empty sections.
- Configured personality applies to chat AI commands through a system message;
  summaries add higher-priority factual constraints instead of mediation criteria.
  Custom personalities replace the default voice rather than having sarcasm forced on them.
  Invocation-specific
  mediation goals are escaped, quoted user input, separate from transcript fields,
  and never placed in system messages. Chat transcripts are explicitly untrusted
  user input. This reduces instruction confusion but
  cannot eliminate prompt injection or make AI judgments authoritative.
- Each chat keeps the newest `buffer_size` supported messages in memory, storing
  author, text/caption, and bounded resource references. Messages handled as commands or command-usage
  feedback are not buffered; unknown commands can still enter the buffer.
  Unsupported media/service messages and blank messages without attachments are skipped. Each entry
  is capped at 50,000 Unicode characters (author at 256), before on-demand enrichment. Transcript author/text
  fields are quoted and control characters are escaped to avoid multiline speaker
  spoofing. Mediation and summaries retain the newest selected whole messages in chronological order
  within 50,000 rendered characters. If the newest message alone exceeds the cap,
  only its text is shortened from the start, preserving its author and newest
  content with an ellipsis. Display names are not verified identities. These are character
  limits, not token budgets; a selected model can still reject a request.
- Buffers are per chat, **not per topic**, shared across that chat's forum topics.
  They are lost on restart, with no persistence, expiry, or global chat-count
  bound. Size the buffer and deployment appropriately for the groups you serve.
- AI answers from `/ask`, `/summary`, and `/mediate` render basic Markdown as Telegram formatting.
  Bold, italic, strikethrough, links, inline code, code blocks, headings, and lists are supported.
  Headings use bold text. Raw HTML remains literal, and links accept only HTTP/HTTPS without credentials.
  Formatting uses explicit Telegram entities, not Telegram MarkdownV2 parsing.
  Entity ranges use UTF-16 units and continue across message boundaries.
  Each chunk sends at most 100 entities. Additional formatting is dropped without dropping text.
  If Telegram rejects the entities, the bot retries that chunk without formatting.
  Transcriptions, help, moderation feedback, and processing notices remain plain text.
- Replies are split into chunks of at most 4096 UTF-16 units,
  preserving Unicode and meaningful chunk content. Whitespace-only chunks are
  skipped because Telegram rejects them. Chunks are sent sequentially; a thread
  identifier is forwarded only for actual topic messages. An explicit Telegram
  `retry_after` is retried once only when its wait is at most 30 seconds; longer
  waits stop delivery promptly. Delivery failure can
  leave a partial answer; concurrent commands can interleave replies.
- There is no per-user rate limit or spending quota. Group access is restricted by
  `ALLOWED_GROUP_IDS`. Anyone in an allowed group can invoke AI commands and incur costs.

## Development checks

All checks run without Telegram or OpenAI credentials:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo +1.96.0 test --locked
```

CI is configured to run these checks on stable Rust and the locked tests on
Rust 1.96.0. Configuring CI does not imply a remote workflow run has passed.

## Potential next features (not implemented)

Useful follow-up work, subject to separate design and authorization:

- **Consent and cost controls:** per-user quotas, an administrator
  AI toggle, and visible data-retention settings.
- **Summary enhancements:** links to source messages and topic-specific context,
  rather than mixing forum topics.
- **Human-reviewed moderation assistance:** explain suspected rule violations and
  suggest actions for administrators; never auto-ban solely on model output.
- **Translation and accessibility:** opt-in audio translation and richer
  transcription features such as timestamps or speaker labels.
- **Telegram interaction improvements:** registered command menus, inline
  confirmation buttons for destructive actions, and cancellation of slow AI work.
- **Durable operations:** optional encrypted storage, expiry/forget controls,
  budget tracking, and bounded chat retention.

No commands or background behaviors for these proposals are included in this update.
