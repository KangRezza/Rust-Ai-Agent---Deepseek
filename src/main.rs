use std::env;
use std::io::Write;
use std::path::Path;
use std::fs::File;
use std::net::SocketAddr;
use clap::Parser;
use colored::Colorize;
use dotenv::dotenv;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::history::DefaultHistory;
use axum::serve;
use tokio::net::TcpListener;

use crate::providers::deepseek::DeepSeekProvider;
use crate::knowledge_base::knowledge_base::KnowledgeBaseHandler;
use crate::database::Database;
use crate::learning::LearningManager;
use crate::personality::{Personality, PersonalityProfile};

// Twitter integration
use crate::providers::twitter::manager::ConversationManager;

// Web crawler integration
use crate::providers::web_crawler::crawler_manager::WebCrawlerManager;
use crate::providers::web_crawler::WebCrawler;

// Command handling
use crate::commands::CommandHandler;

// Module imports
mod memory;
mod providers;
mod knowledge_base;
mod database;
mod learning;
mod completion;
mod personality;
mod commands;
mod api;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    api_key: Option<String>,

    #[arg(long)]
    twitter: bool,

    #[arg(long)]
    crawler: bool,

    #[arg(long)]
    character: Option<String>,

    #[arg(long)]
    twitter_cookie: Option<String>,

    #[arg(long)]
    twitter_username: Option<String>,

    #[arg(long)]
    twitter_password: Option<String>,

    #[arg(long)]
    twitter_email: Option<String>,

    #[arg(long)]
    api: bool,

    #[arg(long, default_value = "3000")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize colored output
    colored::control::set_override(true);

    // Load environment variables
    dotenv().ok();

    // Parse command line arguments
    let args = Args::parse();

    // Get API key from command line or environment
    let api_key = match &args.api_key {
        Some(key) => key.clone(),
        None => env::var("DEEPSEEK_API_KEY").expect("API key must be provided via --api-key or DEEPSEEK_API_KEY env var"),
    };

    // Initialize personality
    let mut current_personality = if let Some(character_file) = &args.character {
        match load_personality_from_filename(character_file) {
            Some(personality) => personality,
            None => {
                println!("Failed to load character: {}", character_file);
                create_default_personality()
            }
        }
    } else {
        create_default_personality()
    };

    // Extract PersonalityProfile from Personality
    let personality_profile = match &current_personality {
        Personality::Dynamic(profile) => profile.clone(),
    };

    // Initialize Deepseek provider
    let deepseek_provider = DeepSeekProvider::new(
        api_key.clone(),
        personality_profile.generate_system_prompt(),
    ).await?;

    // Initialize database
    let database = Database::new("data/agent.db").await?;

    let result = if args.api {
        run_api_server(args).await
    } else {
        run_cli_mode(
            &args,
            personality_profile,
            deepseek_provider,
            database,
        ).await
    };
    
    result
}

async fn run_cli_mode(
    args: &Args,
    personality_profile: PersonalityProfile,
    deepseek_provider: DeepSeekProvider,
    database: Database,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize knowledge base handler
    let knowledge_base_handler = KnowledgeBaseHandler::new("data/knowledge_base.json");

    // Initialize learning manager
    let learning_manager = LearningManager::new(database.clone(), knowledge_base_handler.clone());

    // Initialize command handler
    let mut command_handler = CommandHandler::new(
        personality_profile.clone(),
        if args.twitter {
            Some(ConversationManager::new(personality_profile.clone()).await?)
        } else {
            None
        },
        if args.crawler {
            Some(WebCrawlerManager::new(personality_profile.clone()).await?)
        } else {
            None
        },
        deepseek_provider,
    ).await?;

    // Show initial help menu
    command_handler.handle_command("help").await?;

    // Initialize rustyline editor
    let mut rl = Editor::<(), DefaultHistory>::new()?;
    
    // Main input loop
    loop {
        match rl.readline("👤 ") {
            Ok(line) => {
                let input = line.trim();
                rl.add_history_entry(input);
                
                if let Err(e) = command_handler.handle_command(input).await {
                    println!("{}", e.red());
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

fn load_personality_from_filename(filename: &str) -> Option<Personality> {
    let path = Path::new("characters").join(filename);
    if path.exists() {
        if let Ok(file) = File::open(path) {
            if let Ok(profile) = serde_json::from_reader::<_, PersonalityProfile>(file) {
                return Some(Personality::Dynamic(profile));
            }
        }
    }
    None
}

fn create_default_personality() -> Personality {
    Personality::Dynamic(PersonalityProfile {
        name: "Helpful Assistant".to_string(),
        attributes: serde_json::json!({
            "description": "a helpful AI coding assistant",
            "style": "professional and technically precise",
            "expertise": "programming, software development, and technical problem-solving",
            "motto": "Always here to help with your coding needs",
            "example_code": [
                "```python\n# Example function\ndef greet(name):\n    return f'Hello, {name}!'\n```",
                "```rust\n// Example struct\nstruct User {\n    name: String,\n    age: u32\n}\n```"
            ]
        }),
    })
}

async fn run_api_server(args: Args) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = format!("0.0.0.0:{}", args.port)
        .parse()
        .expect("Failed to parse address");
    
    println!("Starting API server on {}", addr);
    
    // Get API key from command line or environment
    let api_key = match &args.api_key {
        Some(key) => key.clone(),
        None => env::var("DEEPSEEK_API_KEY").expect("API key must be provided via --api-key or DEEPSEEK_API_KEY env var"),
    };

    // Initialize personality
    let personality = if let Some(character_file) = &args.character {
        if let Some(Personality::Dynamic(profile)) = load_personality_from_filename(character_file) {
            profile
        } else {
            create_default_personality().into_dynamic_profile()
        }
    } else {
        create_default_personality().into_dynamic_profile()
    };
    
    // Initialize providers
    let deepseek = DeepSeekProvider::new(
        api_key.clone(),
        personality.generate_system_prompt(),
    ).await?;
    
    // Initialize database
    let db = Database::new("data/agent.db").await?;
    
    println!("Initializing API routes...");
    let app = crate::api::create_api(deepseek, personality, db).await;
    
    println!("API routes configured, attempting to bind to address...");
    
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;
    
    println!("Server successfully bound to {}", addr);
    println!("Ready to accept connections!");
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}

// Add helper method for Personality
impl Personality {
    fn into_dynamic_profile(self) -> PersonalityProfile {
        match self {
            Personality::Dynamic(profile) => profile,
        }
    }
}
