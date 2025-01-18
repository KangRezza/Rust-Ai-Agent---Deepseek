pub mod memory;
pub mod providers;
pub mod completion;
pub mod knowledge_base;
pub mod database;
pub mod learning;
pub mod personality;
pub mod commands;

// Re-export commonly used items
pub use personality::PersonalityProfile;
pub use providers::web_crawler::crawler_manager::WebCrawlerManager;
pub use providers::document::DocumentProcessor; 