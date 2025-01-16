use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::State,
    response::{IntoResponse, Response},
    http::{Method, header, StatusCode},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, Any};
use std::error::Error;
use std::fmt;
use tokio::fs;

use crate::personality::PersonalityProfile;
use crate::DeepSeekProvider;
use crate::database::Database;
use crate::completion::CompletionProvider;


#[derive(Clone)]
pub struct AppState {
    deepseek: Arc<DeepSeekProvider>,
    personality: Arc<RwLock<PersonalityProfile>>,
    db: Arc<Database>,
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
) -> Router {
    let state = AppState {
        deepseek: Arc::new(deepseek),
        personality: Arc::new(RwLock::new(personality)),
        db: Arc::new(db),
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
