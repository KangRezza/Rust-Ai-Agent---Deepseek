use colored::Colorize;
use crate::providers::deepseek::deepseek::DeepSeekProvider;
use crate::personality::PersonalityProfile;
use crate::providers::twitter::manager::ConversationManager;
use crate::providers::web_crawler::crawler_manager::WebCrawlerManager;
use crate::completion::CompletionProvider;
use crate::memory::{ShortTermMemory, LongTermMemory};
use crate::database::Database;

mod character;
mod twitter;
mod web;
mod system;
mod document;

pub struct CommandHandler {
    twitter_manager: Option<ConversationManager>,
    web_crawler: Option<WebCrawlerManager>,
    deepseek_provider: DeepSeekProvider,
    personality: PersonalityProfile,
    memory: ShortTermMemory,
    db: Database,
    long_term_memory: LongTermMemory,
}

impl CommandHandler {
    pub async fn new(
        personality: PersonalityProfile,
        twitter_manager: Option<ConversationManager>,
        web_crawler: Option<WebCrawlerManager>,
        deepseek_provider: DeepSeekProvider,
    ) -> Result<Self, String> {
        let db = Database::new("agent.db")
            .await
            .map_err(|e| format!("Failed to initialize database: {}", e))?;

        Ok(Self {
            twitter_manager,
            web_crawler,
            deepseek_provider,
            personality,
            memory: ShortTermMemory::new(),
            long_term_memory: LongTermMemory::new(),
            db,
        })
    }

    pub async fn handle_command(&mut self, input: &str) -> Result<(), String> {
        if input.is_empty() {
            return Ok(());
        }

        let input = input.trim();

        // Handle single-word commands first
        match input.to_lowercase().as_str() {
            "help" | "exit" | "quit" => return self.handle_system_command(input).await,
            "chars" | "characters" | "load" => return self.handle_character_command(input).await,
            _ => {}
        }

        // Handle command prefixes
        if input.starts_with("load ") {
            return self.handle_character_command(input).await;
        }

        // Document commands
        if input.starts_with("doc ") {
            return document::handle_command(
                input, 
                &self.deepseek_provider,
                &mut self.memory,
                &mut self.long_term_memory,
                &self.db
            ).await;
        }

        // Twitter commands
        if input.starts_with("tweet ") || 
           input.starts_with("autopost ") || 
           input.eq_ignore_ascii_case("tweet") ||
           input.eq_ignore_ascii_case("autopost") ||
           input.starts_with("reply ") || 
           input.starts_with("dm @") {
            return self.handle_twitter_command(input).await;
        }

        // Web commands
        if input.starts_with("analyze ") || 
           input.eq_ignore_ascii_case("analyze") ||
           input.starts_with("research ") ||
           input.eq_ignore_ascii_case("research") ||
           input.starts_with("links ") ||
           input.eq_ignore_ascii_case("links") {
            return self.handle_web_command(input).await;
        }

        // Default to chat completion if no command matches
        self.handle_chat(input).await
    }

    async fn handle_twitter_command(&mut self, input: &str) -> Result<(), String> {
        if input.eq_ignore_ascii_case("tweet") {
            println!("Please provide a message to tweet.");
            println!("Usage: tweet <message>");
            return Ok(());
        }
        if input.eq_ignore_ascii_case("autopost") {
            println!("Please specify start or stop for autopost.");
            println!("Usage: autopost start <minutes> or autopost stop");
            return Ok(());
        }
        twitter::handle_command(input, &mut self.twitter_manager).await
    }

    async fn handle_web_command(&mut self, input: &str) -> Result<(), String> {
        web::handle_command(
            input, 
            &mut self.web_crawler, 
            &self.deepseek_provider,
            &mut self.memory,
            &mut self.long_term_memory,
        ).await
    }

    async fn handle_character_command(&mut self, input: &str) -> Result<(), String> {
        let result = character::handle_command(input, &mut self.personality);
        if result.is_ok() {
            // Update DeepSeek provider with new personality
            if let Err(e) = self.deepseek_provider.update_personality(
                self.personality.generate_system_prompt()
            ).await {
                return Err(format!("Failed to update personality: {}", e));
            }
        }
        result
    }

    async fn handle_system_command(&mut self, input: &str) -> Result<(), String> {
        system::handle_command(input)
    }

    async fn handle_chat(&mut self, input: &str) -> Result<(), String> {
        // Count input tokens
        let input_tokens = input.split_whitespace().count();
        println!("📥 Input tokens: {}", input_tokens.to_string().cyan());

        // Get response from AI
        match self.deepseek_provider.complete(input).await {
            Ok(response) => {
                let response_tokens = response.split_whitespace().count();
                self.print_response("", &response, input_tokens, response_tokens);
                Ok(())
            }
            Err(e) => Err(format!("Failed to get AI response: {}", e))
        }
    }

    fn print_response(&self, _character_name: &str, response: &str, input_tokens: usize, response_tokens: usize) {
        println!("{}", response.truecolor(255, 236, 179));
        
        println!("\n📊 Tokens: 📥 Input: {} | 📤 Response: {} | 📈 Total: {}", 
            input_tokens.to_string().cyan(),
            response_tokens.to_string().cyan(),
            (input_tokens + response_tokens).to_string().cyan()
        );
        println!();
    }
}

pub use document::handle_command as handle_document_command;
