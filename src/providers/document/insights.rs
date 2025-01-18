use serde::{Deserialize, Serialize};
use std::error::Error;
use crate::providers::deepseek::deepseek::DeepSeekProvider;
use crate::completion::CompletionProvider;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Insight {
    pub text: String,
    pub relevance: f32,
}

impl fmt::Display for Insight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Insight: {} (Relevance: {:.2})", self.text, self.relevance)
    }
}

pub struct InsightExtractor {
    deepseek_provider: DeepSeekProvider,
}

impl InsightExtractor {
    pub async fn new(api_key: String, system_message: String) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let deepseek_provider = DeepSeekProvider::new(api_key, system_message).await?;
        Ok(Self { deepseek_provider })
    }

    pub async fn extract_insights(&self, text: &str) -> Result<Vec<Insight>, Box<dyn Error>> {
        // Use AI to extract insights from the text
        let prompt = format!(
            r#"Extract key insights from the following text and format them as a JSON array.

Each insight must be an object with exactly these fields:
"text": (string) The insight text
"relevance": (number) Importance score between 0 and 1

Example format:
[
  {{"text": "First key insight here", "relevance": 0.95}},
  {{"text": "Second key insight here", "relevance": 0.85}}
]

Text to analyze:
{}

Respond ONLY with the JSON array. Do not add any explanations or additional text."#,
            text
        );

        let response = self.deepseek_provider.complete(&prompt).await?;

        // Debug: Print raw response
        eprintln!("Raw AI response:\n{}", response);

        // Clean and parse the response
        let cleaned_response = response
            .trim()
            .trim_matches('`')  // Remove code block markers
            .trim_start_matches("json")  // Remove language identifier
            .trim_start_matches("JSON")
            .replace('\'', "\"")  // Replace single quotes with double quotes
            .trim()
            .to_string();

        // Debug: Print cleaned response
        eprintln!("Cleaned response:\n{}", cleaned_response);

        // Try to parse the cleaned response
        match serde_json::from_str(&cleaned_response) {
            Ok(insights) => Ok(insights),
            Err(_err) => {
                // If JSON parsing fails, try to fix common JSON issues
                let fixed_response = if cleaned_response.starts_with("{") && cleaned_response.ends_with("}") {
                    format!("[{}]", cleaned_response)
                } else if !cleaned_response.starts_with("[") {
                    format!("[{}]", cleaned_response)
                } else {
                    cleaned_response
                };

                // Debug: Print fixed response
                eprintln!("Fixed response:\n{}", fixed_response);

                match serde_json::from_str(&fixed_response) {
                    Ok(insights) => Ok(insights),
                    Err(_) => {
                        // If JSON parsing fails, treat the response as a direct analysis
                        // Split by lines and assign default relevance
                        let insights = response
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| Insight {
                                text: line.trim().to_string(),
                                relevance: 0.8, // Default relevance for direct insights
                            })
                            .collect();
                        Ok(insights)
                    }
                }
            }
        }
    }

    // New method for quick, direct analysis without JSON
    pub async fn quick_analyze(&self, text: &str) -> Result<String, Box<dyn Error>> {
        let prompt = format!(
            "Please analyze this text and provide the key insights in a clear, concise way:\n\n{}",
            text
        );

        let response = self.deepseek_provider.complete(&prompt).await?;
        Ok(response)
    }
}
