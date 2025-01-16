use super::WebCrawler;
use crate::personality::PersonalityProfile;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct WebCrawlerManager {
    crawler: Arc<Mutex<WebCrawler>>,
    profile: PersonalityProfile,

}

impl WebCrawlerManager {
    pub async fn new(profile: PersonalityProfile) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let crawler = WebCrawler::new()?;
        Ok(Self {
            crawler: Arc::new(Mutex::new(crawler)),
            profile,
        })
    }

    pub async fn analyze_webpage(&self, url: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let crawler = self.crawler.lock().await;
        let page = crawler.visit_page(url).await?;
        let mut analysis = Vec::new();

        analysis.push(format!("🔍 Analyzing: {}", url));
        analysis.push(String::new());
        analysis.push("📚 Content Analysis:".to_string());
        analysis.push("──────────────────────────────────────────────────".to_string());
        analysis.push(page.text);
        analysis.push("──────────────────────────────────────────────────".to_string());

        Ok(analysis)
    }

    pub async fn research_topic(&self, topic: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let crawler = self.crawler.lock().await;
        let search_results = crawler.search(topic).await?;
        
        let mut findings = Vec::new();
        findings.push(format!("📚 Research Results for: {}", topic));
        findings.push("──────────────────────────────────────────────────".to_string());
        findings.push("🔍 Search Results:".to_string());

        for (i, url) in search_results.iter().enumerate() {
            findings.push(format!("{}. {}", i + 1, url));
        }

        findings.push("──────────────────────────────────────────────────".to_string());
        findings.push("💡 Tip: Click any of these links to read more about the topic".to_string());
        findings.push(format!("📊 Found {} sources", search_results.len()));

        Ok(findings)
    }

    pub async fn follow_links(&self, url: &str, _depth: u32) -> Result<String, Box<dyn Error + Send + Sync>> {
        let crawler = self.crawler.lock().await;
        let page = crawler.visit_page(url).await?;
        
        let mut result = String::new();
        result.push_str(&format!("🔗 Links found on: {}\n", url));
        result.push_str("──────────────────────────────────────────────────\n");

        if page.links.is_empty() {
            result.push_str("No links found on this page.\n");
        } else {
            for (i, link) in page.links.iter().enumerate() {
                result.push_str(&format!("{}. {}\n", i + 1, link));
            }
            result.push_str(&format!("\nTotal links found: {}\n", page.links.len()));
        }

        result.push_str("──────────────────────────────────────────────────");
        Ok(result)
    }
}
