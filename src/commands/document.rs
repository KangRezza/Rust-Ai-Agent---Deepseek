use crate::providers::deepseek::deepseek::DeepSeekProvider;
use crate::providers::document::DocumentProcessor;
use crate::providers::document::insights::Insight;
use crate::completion::CompletionProvider;
use crate::memory::{ShortTermMemory, LongTermMemory};
use crate::database::Database;
use colored::Colorize;
use std::path::Path;

pub async fn handle_command(
    input: &str, 
    provider: &DeepSeekProvider,
    memory: &mut ShortTermMemory,
    long_term_memory: &mut LongTermMemory,
    db: &Database
) -> Result<(), String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() < 2 {
        println!("📚 Document Commands:");
        println!("  doc analyze <file_path>   - Detailed analysis of document");
        println!("  doc summary <file_path>   - Quick summary");
        println!("  doc extract <file_path>   - Extract text only");
        println!("  doc ocr <image_path>      - Extract text from image");
        println!("  doc batch <folder_path>   - Process multiple files");
        println!("  doc info <file_path>      - Show file information");
        return Ok(());
    }

    let command = parts[1];
    let file_path = parts.get(2).ok_or("Missing file path")?;

    match command {
        "analyze" => {
            println!("📄 Analyzing document: {}", file_path.bright_yellow());
            
            let insights = process_document(file_path, provider).await?;

            // Store document context in memory
            memory.add_interaction(
                &format!("Document being discussed: {}", file_path),
                &format!("Document insights:\n{}", 
                    insights.iter()
                        .map(|i| format!("• {}", i.text))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            );

            // Store high-relevance insights in long-term memory
            for insight in &insights {
                if insight.relevance > 0.7 {
                    long_term_memory.add_memory(
                        &format!("Document: {}", file_path),
                        &format!("Key insight: {}", insight.text)
                    );
                }
            }

            // Store in database for persistence
            for insight in &insights {
                if let Err(e) = db.save_document_insight(
                    file_path.to_string(),
                    insight.text.clone(),
                    insight.relevance,
                    "analysis".to_string()
                ).await {
                    eprintln!("Warning: Failed to save insight to database: {}", e);
                }
            }

            // Get character-specific analysis
            let analysis_prompt = format!(
                "{}\n\nAs this character, analyze these document insights and provide your unique perspective. \
                Consider your personality traits and expertise when providing this analysis. \
                Be creative and stay true to your character's style. \
                After your analysis, invite further questions about the document:\n\n{}",
                provider.get_system_message(),
                insights.iter()
                    .map(|i| format!("• {}", i.text))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            let analysis = provider.complete(&analysis_prompt).await
                .map_err(|e| format!("Failed to generate analysis: {}", e))?;

            println!("\n📊 Analysis Results:");
            println!("{}", analysis.bright_green());
            println!("\n💭 You can now ask questions about the document or request more specific analysis.");
            Ok(())
        },
        "chat" => {
            // Get the chat query from remaining parts
            let query = parts[2..].join(" ");
            
            // Build context from memory
            let context = memory.get_interactions()
                .iter()
                .take(5)
                .map(|(input, response)| format!("User: {}\nAssistant: {}", input, response))
                .collect::<Vec<_>>()
                .join("\n\n");

            // Create chat prompt with context
            let chat_prompt = format!(
                "{}\n\nPrevious context:\n{}\n\nUser question about the document: {}\n\n\
                Answer the question based on the document context while maintaining your character's personality.",
                provider.get_system_message(),
                context,
                query
            );

            let response = provider.complete(&chat_prompt).await
                .map_err(|e| format!("Failed to get response: {}", e))?;

            // Store the interaction
            memory.add_interaction(&query, &response);

            println!("\n💬 Response:");
            println!("{}", response.bright_green());
            Ok(())
        },
        "summary" => {
            println!("📝 Generating summary for: {}", file_path.bright_yellow());
            
            let insights = process_document(file_path, provider).await?;

            // Create a personality-aware summary prompt
            let summary_prompt = format!(
                "{}\n\nAs this character, provide a concise summary of these document insights. \
                Use your unique personality traits and communication style. \
                Make the summary reflect your character's perspective and expertise:\n\n{}",
                provider.get_system_message(), // Include character's personality
                insights.iter()
                    .map(|i| format!("• {}", i.text))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            let summary = provider.complete(&summary_prompt).await
                .map_err(|e| format!("Failed to generate summary: {}", e))?;

            println!("\n📋 Summary:");
            println!("{}", summary.bright_green());
            Ok(())
        },
        "extract" => {
            println!("📄 Extracting text from: {}", file_path.bright_yellow());
            
            let insights = process_document(file_path, provider).await?;

            println!("\n📝 Extracted Text:");
            for insight in insights {
                println!("{}", insight.text);
            }
            Ok(())
        },
        "ocr" => process_image(file_path, provider).await,
        "batch" => process_batch(file_path, provider).await,
        "info" => show_file_info(file_path).await,
        _ => Err(format!("Unknown document command: {}", command))
    }
}

async fn process_image(file_path: &str, provider: &DeepSeekProvider) -> Result<(), String> {
    println!("🔍 Processing image: {}", file_path.bright_yellow());
    
    let api_key = provider.get_api_key().to_string();
    let system_message = provider.get_system_message().to_string();
    let mut processor = DocumentProcessor::new(api_key, system_message)
        .await
        .map_err(|e| e.to_string())?;

    let insights = processor.process_document(file_path).await
        .map_err(|e| format!("Failed to process image: {}", e))?;

    // Create a personality-aware OCR analysis prompt
    let analysis_prompt = format!(
        "{}\n\nAs this character, analyze this OCR text and provide insights in your unique style:\n\n{}",
        provider.get_system_message(),
        insights.iter()
            .map(|i| i.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let analysis = provider.complete(&analysis_prompt).await
        .map_err(|e| format!("Failed to analyze OCR text: {}", e))?;

    println!("\n📝 Analysis:");
    println!("{}", analysis.bright_green());
    Ok(())
}

async fn process_batch(folder_path: &str, provider: &DeepSeekProvider) -> Result<(), String> {
    use tokio::fs;
    use indicatif::{ProgressBar, ProgressStyle};

    println!("📁 Processing files in: {}", folder_path.bright_yellow());

    let mut entries = fs::read_dir(folder_path).await
        .map_err(|e| format!("Failed to read directory: {}", e))?;
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} [{elapsed_precise}] {msg}")
        .unwrap());

    let api_key = provider.get_api_key().to_string();
    let system_message = provider.get_system_message().to_string();
    let mut processor = DocumentProcessor::new(api_key, system_message)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| format!("Failed to read entry: {}", e))? 
    {
        let path = entry.path();
        if path.is_file() {
            pb.set_message(format!("Processing {}", path.display()));
            if let Ok(insights) = processor.process_document(path.to_str().unwrap()).await {
                println!("\n📄 {}: {} insights", path.display(), insights.len());
            }
            pb.inc(1);
        }
    }

    pb.finish_with_message("Processing complete");
    Ok(())
}

async fn show_file_info(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to get file info: {}", e))?;

    println!("\n📄 File Information:");
    println!("Name: {}", path.file_name().unwrap().to_string_lossy().bright_yellow());
    println!("Type: {}", path.extension().unwrap_or_default().to_string_lossy().bright_cyan());
    println!("Size: {} bytes", metadata.len().to_string().bright_green());
    println!("Last modified: {}", metadata.modified()
        .map(|time| time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_string())
        .unwrap_or_else(|_| "Unknown".to_string())
    );

    Ok(())
}

// Helper function to process document
async fn process_document(file_path: &str, provider: &DeepSeekProvider) -> Result<Vec<Insight>, String> {
    let api_key = provider.get_api_key().to_string();
    let system_message = provider.get_system_message().to_string();
    let mut processor = DocumentProcessor::new(api_key, system_message)
        .await
        .map_err(|e| e.to_string())?;

    processor.process_document(file_path).await
        .map_err(|e| format!("Failed to process document: {}", e))
}
