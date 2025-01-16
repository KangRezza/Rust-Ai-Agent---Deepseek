use colored::Colorize;

use crate::providers::deepseek::DeepSeekProvider;
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
        self.handle_chat_command(input).await
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
        match input.to_lowercase().as_str() {
            "analyze" => {
                println!("Please provide a URL to analyze.");
                println!("Usage: analyze <url>");
                Ok(())
            }
            "research" => {
                println!("Please provide a topic to research.");
                println!("Usage: research <topic>");
                Ok(())
            }
            "links" => {
                println!("Please provide a URL to extract links from.");
                println!("Usage: links <url>");
                Ok(())
            }
            _ => web::handle_command(input, &mut self.web_crawler).await
        }
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

    async fn handle_chat_command(&mut self, input: &str) -> Result<(), String> {
        // Count input tokens
        let input_tokens = input.split_whitespace().count();
        println!("📥 Input tokens: {}", input_tokens.to_string().cyan());

        // Get recent conversations from database
        let recent_convos = match self.db.get_recent_conversations(5).await {
            Ok(convos) => convos,
            Err(e) => {
                eprintln!("Warning: Failed to get recent conversations: {}", e);
                vec![]
            }
        };
        
        // Build context from recent conversations
        let mut context = String::new();
        for (_timestamp, user_msg, ai_msg, personality) in recent_convos {
            if personality == self.personality.name {
                context.push_str(&format!("User: {}\nAI: {}\n", user_msg, ai_msg));
            }
        }

        // Get response from AI with context
        let prompt = if context.is_empty() {
            input.to_string()
        } else {
            format!("Previous conversation:\n{}\n\nCurrent message: {}", context, input)
        };

        match self.deepseek_provider.complete(&prompt).await {
            Ok(response) => {
                // Count response tokens
                let response_tokens = response.split_whitespace().count();
                
                // Save to database
                if let Err(e) = self.db.save_conversation(
                    input.to_string(), 
                    response.clone(),
                    self.personality.name.clone()
                ).await {
                    eprintln!("Warning: Failed to save conversation to database: {}", e);
                }

                // Print response with token counts
                self.print_response(&self.personality.name, &response, input_tokens, response_tokens);
                Ok(())
            }
            Err(e) => Err(format!("Failed to get AI response: {}", e))
        }
    }

    fn print_response(&self, character_name: &str, response: &str, input_tokens: usize, response_tokens: usize) {
        println!("{}", response.bright_green());
        println!("\n📊 Tokens: 📥 Input: {} | 📤 Response: {} | 📈 Total: {}", 
            input_tokens.to_string().cyan(),
            response_tokens.to_string().cyan(),
            (input_tokens + response_tokens).to_string().cyan()
        );
        println!();
    }
}