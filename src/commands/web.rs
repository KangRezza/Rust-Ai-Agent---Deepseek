use crate::providers::web_crawler::crawler_manager::WebCrawlerManager;
use colored::Colorize;

pub async fn handle_command(
    input: &str,
    crawler: &mut Option<WebCrawlerManager>
) -> Result<(), String> {
    if let Some(ref mut crawler) = crawler {
        if input.starts_with("analyze ") {
            let url = input.trim_start_matches("analyze ").trim();
            if url.is_empty() {
                println!("Please provide a URL to analyze.");
                println!("Usage: analyze <url>");
                return Ok(());
            }
            crawler.analyze_webpage(url).await
                .map(|analysis| {
                    for line in analysis {
                        match line.chars().next() {
                            Some('🔍') => println!("{}", line.bright_cyan()),
                            Some('📑') => println!("{}", line.bright_yellow()),
                            Some('📝') => println!("{}", line.bright_green()),
                            Some('─') => println!("{}", line.bright_black()),
                            _ => println!("  • {}", line),
                        }
                    }
                })
                .map_err(|e| format!("Error analyzing webpage: {}", e))
        }
        else if input.starts_with("research ") {
            let topic = input.trim_start_matches("research ").trim();
            if topic.is_empty() {
                println!("Please provide a topic to research.");
                println!("Usage: research <topic>");
                return Ok(());
            }
            crawler.research_topic(topic).await
                .map(|findings| {
                    for finding in findings {
                        match finding.chars().next() {
                            Some('📚') => println!("\n{}", finding.bright_cyan()),
                            Some('🔍') => println!("\n{}", finding.bright_yellow()),
                            Some('💡') => println!("\n{}", finding.bright_green()),
                            Some('📊') => println!("\n{}", finding.bright_cyan()),
                            Some('─') => println!("{}", finding.bright_black()),
                            _ => println!("  • {}", finding),
                        }
                    }
                })
                .map_err(|e| format!("Error during research: {}", e))
        }
        else if input.starts_with("links ") {
            let url = input.trim_start_matches("links ").trim();
            if url.is_empty() {
                println!("Please provide a URL to extract links from.");
                println!("Usage: links <url>");
                return Ok(());
            }
            crawler.follow_links(url, 1).await
                .map(|result| {
                    for line in result.lines() {
                        match line.chars().next() {
                            Some('🔗') => println!("\n{}", line.bright_cyan()),
                            Some('T') if line.starts_with("Total") => println!("\n{}", line.bright_yellow()),
                            Some('─') => println!("{}", line.bright_black()),
                            _ => println!("  • {}", line),
                        }
                    }
                })
                .map_err(|e| format!("Error following links: {}", e))
        }
        else {
            Err("Unknown web command. Available commands:\n  analyze <url> - Analyze webpage content\n  research <topic> - Research a topic\n  links <url> - Extract links from webpage".to_string())
        }
    } else {
        Err("Web crawler is not initialized. Use --crawler flag to enable it.".to_string())
    }
}