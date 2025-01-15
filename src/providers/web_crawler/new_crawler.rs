use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::time::Duration;
use tokio::time;
use urlencoding;

const DEFAULT_TIMEOUT: u64 = 5;
const MAX_REDIRECTS: usize = 2;
const RATE_LIMIT_DELAY: u64 = 1;
const USER_AGENT: &str = "Mozilla/5.0 (compatible; AIAgent/1.0)";

#[derive(Debug, Clone)]
pub struct WebCrawler {
    client: Client,
    last_visit: std::time::Instant,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageContent {
    pub url: String,
    pub title: Option<String>,
    pub text: String,
    pub links: Vec<String>,
}

impl WebCrawler {
    pub fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT))
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .build()?;

        Ok(Self {
            client,
            last_visit: std::time::Instant::now(),
        })
    }

    async fn rate_limit(&self) {
        let elapsed = self.last_visit.elapsed();
        if elapsed < Duration::from_secs(RATE_LIMIT_DELAY) {
            time::sleep(Duration::from_secs(RATE_LIMIT_DELAY) - elapsed).await;
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        self.rate_limit().await;

        // Return multiple search variations for better results
        Ok(vec![
            // Google variations
            format!("https://www.google.com/search?q={}", urlencoding::encode(query)),
            format!("https://www.google.com/search?q={}&tbm=nws", urlencoding::encode(query)), // News
            format!("https://www.google.com/search?q={}+review", urlencoding::encode(query)),  // Reviews
            
            // DuckDuckGo variations
            format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
            format!("https://duckduckgo.com/?q={}+guide", urlencoding::encode(query)),        // Guides
            format!("https://duckduckgo.com/?q={}&t=h_", urlencoding::encode(query)),         // Different region
            format!("https://duckduckgo.com/?q={}+tutorial", urlencoding::encode(query))      // Tutorials
        ])
    }

    pub async fn visit_page(&self, url: &str) -> Result<PageContent, Box<dyn Error + Send + Sync>> {
        self.rate_limit().await;

        let response = self.client
            .get(url)
            .send()
            .await?;

        let final_url = response.url().to_string();
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|title| title.text().collect::<String>());

        // Extract main content
        let content_selector = Selector::parse("p, h1, h2, h3, ul, ol, li").unwrap();
        let mut text = String::new();
        for element in document.select(&content_selector) {
            let element_text = element.text().collect::<Vec<_>>().join(" ");
            if !element_text.trim().is_empty() {
                text.push_str(&format!("- {}\n", element_text));
            }
        }

        // Extract links
        let link_selector = Selector::parse("a[href]").unwrap();
        let links: Vec<String> = document
            .select(&link_selector)
            .filter_map(|element| element.value().attr("href"))
            .filter(|href| href.starts_with("http"))
            .map(|href| href.to_string())
            .collect();

        // Format output
        println!("\n# Page Analysis");
        if let Some(title) = &title {
            println!("## Title: {}\n", title);
        }
        println!("## URL: {}\n", final_url);
        println!("## Content:\n{}
", text);
        println!("## Links:");
        for link in &links {
            println!("- {}", link);
        }

        Ok(PageContent {
            url: final_url,
            title,
            text,
            links,
        })
    }
}