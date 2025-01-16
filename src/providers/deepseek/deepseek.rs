use reqwest::Client;
use serde_json::json;
use std::env;
use crate::completion::CompletionProvider;
use dotenv::dotenv;

#[derive(Debug)]
pub struct DeepSeekError(Box<dyn std::error::Error + Send + Sync>);

impl std::fmt::Display for DeepSeekError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DeepSeekError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

pub struct DeepSeekProvider {
    api_key: String,
    client: Client,
    system_message: String,
    api_url: String,
}

impl DeepSeekProvider {
    pub async fn new(api_key: String, system_message: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        dotenv().ok();
        let api_url = env::var("DEEPSEEK_BASE_URL")
            .expect("DEEPSEEK_BASE_URL must be set in environment");

        Ok(Self {
            api_key,
            client: Client::new(),
            system_message,
            api_url,
        })
    }

    pub async fn update_personality(&mut self, system_message: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.system_message = system_message;
        Ok(())
    }

    pub async fn get_personality(&self) -> &String {
        &self.system_message
    }

    pub fn update_system_prompt(&mut self, new_prompt: String) {
        self.system_message = new_prompt;
    }
}

#[async_trait::async_trait]
impl CompletionProvider for DeepSeekProvider {
    type Error = DeepSeekError;

    async fn complete(&self, prompt: &str) -> Result<String, DeepSeekError> {
        let api_endpoint = format!("{}/v1/chat/completions", self.api_url);

        let messages = json!([
            {
                "role": "system",
                "content": &self.system_message
            },
            {
                "role": "user",
                "content": prompt
            }
        ]);

        let request_body = json!({
            "model": env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string()),
            "messages": messages,
            "max_tokens": env::var("DEEPSEEK_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(2048),
            "temperature": env::var("DEEPSEEK_TEMPERATURE")
                .ok()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.0),
            "frequency_penalty": 2.0,
            "presence_penalty": 1.5
        });

        let response = self.client
            .post(&api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| DeepSeekError(Box::new(e)))?;

        let response_text = response.text().await
            .map_err(|e| DeepSeekError(Box::new(e)))?;

        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| DeepSeekError(Box::new(e)))?;

        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| DeepSeekError(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to extract content from response: {}", response_text)
            ))))?
            .to_string();

        Ok(content)
    }
}
