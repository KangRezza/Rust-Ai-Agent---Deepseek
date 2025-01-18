use crate::providers::web_crawler::crawler_manager::WebCrawlerManager;
use crate::completion::CompletionProvider;
use crate::providers::deepseek::deepseek::DeepSeekProvider;
use crate::memory::{ShortTermMemory, LongTermMemory};
use colored::Colorize;

pub async fn handle_command(
    input: &str,
    crawler: &mut Option<WebCrawlerManager>,
    provider: &DeepSeekProvider,
    memory: &mut ShortTermMemory,
    long_term_memory: &mut LongTermMemory,
) -> Result<(), String> {
    if let Some(crawler) = crawler {
        match input {
            s if s.starts_with("analyze ") => {
                let url = s.trim_start_matches("analyze ").trim();
                if url.is_empty() {
                    println!("Please provide a URL to analyze.");
                    println!("Usage: analyze <url>");
                    return Ok(());
                }

                let content = crawler.analyze_url(url).await
                    .map_err(|e| format!("Failed to analyze webpage: {}", e))?;

                // Store webpage content in memory
                memory.add_interaction(
                    &format!("Webpage being discussed: {}", url),
                    &format!("Content:\n{}", content)
                );

                // Create personality-aware analysis prompt
                let analysis_prompt = format!(
                    "{}\n\nAs this character, analyze this webpage content and provide your unique perspective. \
                    Consider your personality traits and expertise when providing this analysis. \
                    Be creative and stay true to your character's style:\n\n{}",
                    provider.get_system_message(),
                    content
                );

                let analysis = provider.complete(&analysis_prompt).await
                    .map_err(|e| format!("Failed to analyze content: {}", e))?;

                // Store analysis in memory
                memory.add_interaction(
                    &format!("Analysis of webpage: {}", url),
                    &analysis
                );

                println!("\n📊 Analysis Results for {}:", url.bright_yellow());
                println!("{}", analysis.truecolor(255, 236, 179));
                println!("\n💭 You can now ask questions about this webpage. Try:");
                println!("  web chat what are the main points?");
                println!("  web chat can you explain [specific topic] in more detail?");
                Ok(())
            },
            s if s.starts_with("research ") => {
                let topic = s.trim_start_matches("research ").trim();
                if topic.is_empty() {
                    println!("Please provide a topic to research.");
                    println!("Usage: research <topic>");
                    return Ok(());
                }

                let results = crawler.research_topic(topic).await
                    .map_err(|e| format!("Failed to research topic: {}", e))?;

                // Store research results in memory
                memory.add_interaction(
                    &format!("Research topic: {}", topic),
                    &format!("Research findings:\n{}", results.join("\n"))
                );

                // Create personality-aware research prompt with better structure
                let research_prompt = format!(
                    "{}\n\n\
                    As this character, analyze and synthesize the research about '{}'in your unique style. \
                    Structure your response in these sections:\n\
                    1. Key Findings (3-10 main points)\n\
                    2. Analysis with (your unique perspective)\n\
                    Keep each section focused and concise. \
                    Stay true to your character's expertise and communication style.\n\n\
                    Research content (1 - 5 points) and then make summarize,short and concise with your style:\n{}", 
                    provider.get_system_message(),
                    topic,
                    results.join("\n")
                );

                let analysis = provider.complete(&research_prompt).await
                    .map_err(|e| format!("Failed to synthesize research: {}", e))?;

                // Store analysis in memory
                memory.add_interaction(
                    &format!("Research analysis: {}", topic),
                    &analysis
                );

                println!("\n📚 Research Results for '{}':", topic.bright_yellow());
                println!("{}", analysis.truecolor(255, 236, 179));
                println!("\n💭 You can now ask questions about this research. Try:");
                println!("  web chat tell me more about [specific finding]");
                println!("  web chat what are the implications of [topic]?");
                Ok(())
            },
            s if s.starts_with("links ") => {
                let url = s.trim_start_matches("links ").trim();
                if url.is_empty() {
                    println!("Please provide a URL to extract links from.");
                    println!("Usage: links <url>");
                    return Ok(());
                }

                let links = crawler.extract_links(url).await
                    .map_err(|e| format!("Failed to extract links: {}", e))?;

                println!("\n🔗 Links from {}:", url.bright_yellow());
                let link_count = links.len();
                for link in links {
                    println!("• {}", link);
                }
                println!("\n📊 Total links found: {}", link_count);
                Ok(())
            },
            s if s.starts_with("chat ") => {
                let query = s.trim_start_matches("chat ").trim();
                
                // Get recent context from memory
                let context = memory.get_interactions()
                    .iter()
                    .take(5)
                    .map(|(input, response)| format!("Context: {}\nResponse: {}", input, response))
                    .collect::<Vec<_>>()
                    .join("\n\n");

                // Create chat prompt with context
                let chat_prompt = format!(
                    "{}\n\n\
                    Previous context:\n{}\n\n\
                    User question: {}\n\n\
                    Answer the question based on the previous context while maintaining your character's personality. \
                    Keep your response focused and relevant to the topic being discussed.",
                    provider.get_system_message(),
                    context,
                    query
                );

                let response = provider.complete(&chat_prompt).await
                    .map_err(|e| format!("Failed to get response: {}", e))?;

                // Store the chat interaction
                memory.add_interaction(query, &response);

                println!("\n💬 Response:");
                println!("{}", response.bright_green());
                Ok(())
            },
            _ => Err("Unknown web command. Available commands:\n  analyze <url> - Analyze webpage content\n  research <topic> - Research a topic\n  links <url> - Extract links from webpage".to_string())
        }
    } else {
        Err("Web crawler not initialized. Use --crawler flag to enable web features.".to_string())
    }
}