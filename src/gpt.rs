use crate::bot::Messages;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use std::{env, error::Error};

pub async fn ask(question: String) -> Result<String, Box<dyn Error>> {
    let client = init_gpt_client()?;

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .max_tokens(512u16)
        .messages(vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(crate::consts::PERSONALITY)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(question)
                .build()?
                .into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;

    match &response.choices[0].message.content {
        Some(msg) => Ok(String::from(msg)),
        None => Err("No response given".into()),
    }
}

pub async fn mediate(messages: Messages) -> Result<String, Box<dyn Error>> {
    let messages_lock = messages.read().await;

    let texts_array: Vec<(String, String)> = messages_lock
        .iter()
        .map(|message| {
            (
                message.from().unwrap().full_name(),
                String::from(message.text().unwrap()),
            )
        })
        .collect();

    Ok(format!("{:#?}", texts_array))
}

fn init_gpt_client() -> Result<Client<OpenAIConfig>, Box<dyn Error>> {
    let config = OpenAIConfig::new()
        .with_api_key(env::var("OPENAI_API_KEY")?)
        .with_org_id(env::var("OPENAI_ORG_ID")?);

    Ok(Client::with_config(config))
}
