use std::error::Error;
use async_openai::types::{ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs};
use async_openai::{config::OpenAIConfig, types::CreateChatCompletionRequestArgs};
use async_openai::Client;

pub struct Gpt{
    pub client: Client<OpenAIConfig>,
}

impl Gpt{
    pub fn new() -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(std::env::var("OPENAI_API_KEY").unwrap())
            .with_org_id("org-zaJwWYJ1I0EN4RY2RzNCazSQ");
        let client = Client::with_config(config.clone());
        Self { client }
    } 

    pub async fn ask(&self, prompt: String) -> Result<String, Box<dyn Error>>{
        let request = CreateChatCompletionRequestArgs::default()
            .model("gpt-3.5-turbo")
            .max_tokens(512u16)
            .messages(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("Eres un asistente andaluz con jerga informal y algo irónica. Ayudas a todo aquel que te necesite, no sin antes quejarte un poco, ya que eres algo vago.")
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default() 
                    .content(prompt)
                    .build()?
                    .into(),
            ])
            .build()?;

        let response = self.client
            .chat()
            .create(request)
            .await?;

        if let Some(msg) = &response.choices[0].message.content{
            Ok(msg.to_string())
        } else {
            Err("No message content found".into())
        }
    }
}
