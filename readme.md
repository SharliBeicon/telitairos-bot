# A fully functional AI Powered Telegram bot

## Supported commands
You can either:
- `/ask` for a specified question.
- `/mediate` to read the last N messages of a chat group and mitigate an argument.

## Environment variables needed
```
- TELOXIDE_TOKEN= "/* Your Telegram Bot API Key */"
- OPENAI_API_KEY= "/* Your OpenAI API Key */"
- OPENAI_ORG_ID= "/* Your OpenAI Organization ID */"
```

## Basic usage

You need to specify the personality of the bot as well as its criteria when mitigating an argument.
A size for the context of the N last messages of the chat group is also needed.

For a detailed example go to [TelitairoBot]

### Example

```
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

