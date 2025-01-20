use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::State,
    response::{IntoResponse, Response},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, Any};
use std::error::Error;
use std::fmt;

use crate::personality::PersonalityProfile;
use crate::DeepSeekProvider;
use crate::database::Database;
use crate::completion::CompletionProvider;
use crate::providers::web_crawler::crawler_manager::WebCrawlerManager;
use crate::memory::{ShortTermMemory, LongTermMemory};

#[derive(Clone)]
pub struct AppState {
    deepseek: Arc<DeepSeekProvider>,
    personality: Arc<RwLock<PersonalityProfile>>,
    db: Arc<Database>,
    crawler: Arc<RwLock<Option<WebCrawlerManager>>>,
    memory: Arc<RwLock<ShortTermMemory>>,
    long_term_memory: Arc<RwLock<LongTermMemory>>,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    message: String,
    character: Option<String>,
}

#[derive(Deserialize)]
pub struct CharacterRequest {
    character: String,
}

#[derive(Deserialize)]
pub struct WebRequest {
    command: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    response: String,
    tokens: TokenInfo,
}

#[derive(Serialize)]
pub struct TokenInfo {
    input: usize,
    response: usize,
    total: usize,
}

#[derive(Serialize)]
pub struct CharacterResponse {
    status: String,
}

#[derive(Serialize)]
struct ApiResponse {
    status: String,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiResponse>)>;

#[derive(Debug)]
struct ApiError(String);

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ApiError {}

pub async fn create_api(
    deepseek: DeepSeekProvider,
    personality: PersonalityProfile,
    db: Database,
    crawler: Option<WebCrawlerManager>,
    memory: ShortTermMemory,
    long_term_memory: LongTermMemory,
) -> Router {
    let state = AppState {
        deepseek: Arc::new(deepseek),
        personality: Arc::new(RwLock::new(personality)),
        db: Arc::new(db),
        crawler: Arc::new(RwLock::new(crawler)),
        memory: Arc::new(RwLock::new(memory)),
        long_term_memory: Arc::new(RwLock::new(long_term_memory)),
    };

    println!("Setting up API server with CORS...");

    // Fully permissive CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
        .max_age(std::time::Duration::from_secs(3600));

    println!("CORS configured with permissive settings");

    Router::new()
        .route("/chat", post(chat_handler))
        .route("/character", post(character_handler))
        .route("/health", get(health_check))
        .route("/web", post(web_handler))
        .layer(cors)
        .with_state(state)
}

async fn chat_handler(
    State(mut state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Response {
    let input_tokens = request.message.split_whitespace().count();
    
    // Get recent conversations from database
    let recent_convos = match state.db.get_recent_conversations(5).await {
        Ok(convos) => convos,
        Err(e) => {
            eprintln!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { status: "Database error".to_string() })
            ).into_response();
        }
    };
    
    // Get current personality and build context
    let personality = state.personality.read().await;
    println!("Generating response as character: {}", personality.name);
    
    // Create new DeepSeek provider with current system prompt
    let system_prompt = personality.generate_system_prompt();
    let api_key = match std::env::var("DEEPSEEK_API_KEY") {
        Ok(key) => key,
        Err(e) => {
            eprintln!("API key error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { status: "API key error".to_string() })
            ).into_response();
        }
    };
    
    let new_provider = match DeepSeekProvider::new(api_key, system_prompt).await {
        Ok(provider) => provider,
        Err(e) => {
            eprintln!("Failed to create provider: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { status: "Failed to create provider".to_string() })
            ).into_response();
        }
    };
    
    state.deepseek = Arc::new(new_provider);
    
    let mut context = String::new();
    for (_timestamp, user_msg, ai_msg, pers_name) in recent_convos {
        if pers_name == personality.name {
            context.push_str(&format!("User: {}\nAI: {}\n", user_msg, ai_msg));
        }
    }

    // Create prompt with context
    let prompt = if context.is_empty() {
        request.message.clone()
    } else {
        format!("Previous conversation:\n{}\n\nCurrent message: {}", context, request.message)
    };

    // Get AI response using current personality's system prompt
    let response = match state.deepseek.complete(&prompt).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("AI error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse { status: format!("AI error: {}", e) })
            ).into_response();
        }
    };

    let response_tokens = response.split_whitespace().count();
    
    // Save conversation to database with current personality
    if let Err(e) = state.db.save_conversation(
        request.message.clone(),
        response.clone(),
        personality.name.clone(),
    ).await {
        eprintln!("Warning: Failed to save conversation to database: {}", e);
    }

    Json(ChatResponse {
        response,
        tokens: TokenInfo {
            input: input_tokens,
            response: response_tokens,
            total: input_tokens + response_tokens,
        },
    }).into_response()
}

async fn character_handler(
    State(mut state): State<AppState>,
    Json(request): Json<CharacterRequest>
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
    println!("Changing character to: {}", request.character);
    
    let file_path = format!("characters/{}.json", request.character);
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => {
            match serde_json::from_str::<PersonalityProfile>(&content) {
                Ok(profile) => {
                    // Update the personality
                    *state.personality.write().await = profile;
                    
                    // Create new DeepSeek provider with updated system prompt
                    let system_prompt = state.personality.read().await.generate_system_prompt();
                    let api_key = std::env::var("DEEPSEEK_API_KEY").map_err(|e| {
                        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                            status: format!("API key error: {}", e)
                        }))
                    })?;
                    
                    let new_provider = match DeepSeekProvider::new(api_key, system_prompt).await {
                        Ok(provider) => provider,
                        Err(e) => {
                            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                                status: format!("Failed to create provider: {}", e)
                            })));
                        }
                    };
                    
                    state.deepseek = Arc::new(new_provider);
                    
                    Ok(Json(ApiResponse { 
                        status: "Character changed successfully".to_string() 
                    }))
                },
                Err(e) => {
                    println!("Error parsing character profile: {}", e);
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                        status: "Error parsing character profile".to_string()
                    })))
                }
            }
        },
        Err(e) => {
            println!("Error reading character file: {}", e);
            Err((StatusCode::NOT_FOUND, Json(ApiResponse {
                status: "Character file not found".to_string()
            })))
        }
    }
}

async fn health_check() -> Response {
    println!("Health check requested");
    Json(ApiResponse { 
        status: "Server is running and healthy".to_string() 
    }).into_response()
}

async fn web_handler(
    State(state): State<AppState>,
    Json(request): Json<WebRequest>,
) -> Response {
    let command = request.command.as_str();
    
    let mut crawler = state.crawler.write().await;
    let mut memory = state.memory.write().await;
    let mut long_term_memory = state.long_term_memory.write().await;
    let personality = state.personality.read().await;
    
    match handle_web_command(
        command,
        &mut crawler,
        &state.deepseek,
        &mut memory,
        &mut long_term_memory,
        &personality
    ).await {
        Ok(result) => Json(ApiResponse { 
            status: result 
        }).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse { status: e })
        ).into_response()
    }
}

async fn handle_web_command(
    command: &str,
    crawler: &mut Option<WebCrawlerManager>,
    provider: &DeepSeekProvider,
    memory: &mut ShortTermMemory,
    long_term_memory: &mut LongTermMemory,
    personality: &PersonalityProfile,
) -> Result<String, String> {
    if let Some(crawler) = crawler {
        match command {
            s if s.starts_with("analyze ") => {
                let url = s.trim_start_matches("analyze ").trim();
                if url.is_empty() {
                    return Err("Please provide a URL to analyze.".to_string());
                }

                let content = crawler.analyze_url(url).await
                    .map_err(|e| format!("Failed to analyze webpage: {}", e))?;

                memory.add_interaction(
                    &format!("Webpage being discussed: {}", url),
                    &format!("Content:\n{}", content)
                );

                // Create new provider with current personality
                let system_prompt = personality.generate_system_prompt();
                let new_provider = DeepSeekProvider::new(provider.get_api_key().to_string(), system_prompt)
                    .await
                    .map_err(|e| format!("Failed to create provider: {}", e))?;

                let analysis_prompt = format!(
                    "{}\n\n\
                    Analyze this webpage content and provide your unique perspective. \
                    Consider your personality traits and expertise. \
                    Be creative and stay true to your character's style:\n\n{}",
                    new_provider.get_system_message(),
                    content
                );

                let analysis = new_provider.complete(&analysis_prompt).await
                    .map_err(|e| format!("Failed to analyze content: {}", e))?;

                memory.add_interaction(
                    &format!("Analysis of webpage: {}", url),
                    &analysis
                );

                Ok(analysis)
            },
            s if s.starts_with("research ") => {
                let topic = s.trim_start_matches("research ").trim();
                if topic.is_empty() {
                    return Err("Please provide a topic to research.".to_string());
                }

                let results = crawler.research_topic(topic).await
                    .map_err(|e| format!("Failed to research topic: {}", e))?;

                memory.add_interaction(
                    &format!("Research topic: {}", topic),
                    &format!("Research findings:\n{}", results.join("\n"))
                );

                // Create new provider with current personality
                let system_prompt = personality.generate_system_prompt();
                let new_provider = DeepSeekProvider::new(provider.get_api_key().to_string(), system_prompt)
                    .await
                    .map_err(|e| format!("Failed to create provider: {}", e))?;

                let research_prompt = format!(
                    "{}\n\n\
                    Analyze and synthesize the research about '{}' in your unique style. \
                    Structure your response in these sections:\n\
                    1. Key Findings (3-10 main points)\n\
                    2. Analysis (from your unique perspective)\n\
                    Keep each section focused and insightful. \
                    Stay true to your character's expertise and communication style.\n\n\
                    3. Then make a quick summary of all of these, short and insightful with your own unique style:\n{}",  
                    new_provider.get_system_message(),
                    topic,
                    results.join("\n")
                );

                let analysis = new_provider.complete(&research_prompt).await
                    .map_err(|e| format!("Failed to synthesize research: {}", e))?;

                memory.add_interaction(
                    &format!("Research analysis: {}", topic),
                    &analysis
                );

                Ok(analysis)
            },
            s if s.starts_with("links ") => {
                let url = s.trim_start_matches("links ").trim();
                if url.is_empty() {
                    return Err("Please provide a URL to extract links from.".to_string());
                }

                let links = crawler.extract_links(url).await
                    .map_err(|e| format!("Failed to extract links: {}", e))?;

                Ok(format!("Links found:\n{}", links.join("\n")))
            },
            _ => Err("Unknown web command. Available commands: analyze <url>, research <topic>, links <url>".to_string())
        }
    } else {
        Err("Web crawler not initialized. Use --crawler flag to enable web features.".to_string())
    }
} 