# A fully functional AI Powered Telegram bot

## Setup guide
1. Install Rust with [rustup](http://rustup.rs/).
2. Create a Telegram bot with [@BotFather](https://t.me/botfather) and get the token
3. Create an [OpenAI API Platform account](https://openai.com/api/) and get both API Token and Organization Id
4. Add environment variables
```bash
- TELOXIDE_TOKEN= "/* Your Telegram Bot API Key */"
- OPENAI_API_KEY= "/* Your OpenAI API Key */"
- OPENAI_ORG_ID= "/* Your OpenAI Organization ID */"
```
5. Put these lines into your Cargo.toml
```toml
[dependencies]
telitairos-bot = "0.1.1"
teloxide = { version = "0.12", features = ["macros"] }
log = "0.4"
pretty_env_logger = "0.4"
tokio = { version =  "1.8", features = ["rt-multi-thread", "macros"] }
```

## Supported commands
You can either:
- `/ask` for a specified question.
- `/mediate` to read the last N messages of a chat group and mitigate an argument.


## Basic usage

You need to specify the personality of the bot as well as its criteria when mitigating an argument.
A size for the context of the N last messages of the chat group is also needed.

For a detailed example go to [TelitairoBot](https://docs.rs/telitairos-bot/latest/telitairos_bot/struct.TelitairoBot.html)
### Example

```rust
#[tokio::main]
async fn main() {
   pretty_env_logger::init();
   log::info!("Starting bot");

   let telitairo_bot = TelitairoBot::new(
       String::from(/*Personality */),
       String::from(/* Mediation criteria */),
       /*size */,
   );

   telitairo_bot.dispatch().await;
}

