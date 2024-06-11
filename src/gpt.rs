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
use string_builder::Builder;

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

// TODO: Better user and message handling
pub async fn mediate(messages: Messages) -> Result<String, Box<dyn Error>> {
    let messages_lock = messages.read().await;

    let mut conversation = Builder::default();
    let _ = messages_lock.iter().map(|message| {
        conversation.append(message.from().unwrap().full_name());
        conversation.append(": ");
        conversation.append(message.text().unwrap());
        conversation.append("\n");
    });
    drop(messages_lock);

    let client = init_gpt_client()?;
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .max_tokens(4096u16)
        .messages(vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(crate::consts::PERSONALITY)
                .build()?
                .into(),
            ChatCompletionRequestSystemMessageArgs::default()
                .content(crate::consts::MEDIATE_QUERY)
                .build()?
                .into(),
            ChatCompletionRequestSystemMessageArgs::default()
                .content(conversation.string().unwrap())
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

fn init_gpt_client() -> Result<Client<OpenAIConfig>, Box<dyn Error>> {
    let config = OpenAIConfig::new()
        .with_api_key(env::var("OPENAI_API_KEY")?)
        .with_org_id(env::var("OPENAI_ORG_ID")?);

    Ok(Client::with_config(config))
}
