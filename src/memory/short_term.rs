// src/memory/short_term.rs
use std::collections::{VecDeque, HashMap};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub timestamp: DateTime<Utc>,
    pub user_input: String,
    pub ai_response: String,
    pub topics: Vec<String>,
    pub relevance_score: f32,
}

pub struct ShortTermMemory {
    conversations: VecDeque<Conversation>,
    topic_index: HashMap<String, Vec<usize>>,
    max_size: usize,
    interactions: Vec<(String, String)>,
}

impl ShortTermMemory {
    pub fn new() -> Self {
        Self {
            conversations: VecDeque::new(),
            topic_index: HashMap::new(),
            max_size: 50, // Keep last 50 conversations
            interactions: Vec::new(),
        }
    }

    pub fn add_interaction(&mut self, user_input: &str, ai_response: &str) {
        self.interactions.push((user_input.to_string(), ai_response.to_string()));
    }

    pub fn get_interactions(&self) -> &Vec<(String, String)> {
        &self.interactions
    }

    fn extract_topics(&self, user_input: &str, ai_response: &str) -> Vec<String> {
        let mut topics = Vec::new();
        let combined_text = format!("{} {}", user_input, ai_response);
        
        // Simple topic extraction (can be enhanced with NLP)
        let words: Vec<&str> = combined_text.split_whitespace().collect();
        for window in words.windows(2) {
            if window[0].len() > 3 && window[1].len() > 3 {
                topics.push(format!("{} {}", window[0], window[1]));
            }
        }

        topics.sort();
        topics.dedup();
        topics
    }

    fn calculate_relevance(&self, topics: &[String]) -> f32 {
        if self.conversations.is_empty() {
            return 1.0;
        }

        let mut relevance = 0.0;
        let recent_conversations: Vec<_> = self.conversations.iter().rev().take(5).collect();

        for topic in topics {
            for conv in &recent_conversations {
                if conv.topics.contains(topic) {
                    relevance += 0.2; // Increase relevance for each topic match
                }
            }
        }

        if relevance > 5.0 {
            5.0
        } else {
            relevance + 1.0
        }
    }

    fn prune_conversations(&mut self) {
        if self.conversations.len() <= self.max_size {
            return;
        }

        // Sort conversations by relevance and recency
        let mut conversations: Vec<_> = self.conversations.drain(..).collect();
        conversations.sort_by(|a, b| {
            let recency_weight = 0.7;
            let relevance_weight = 0.3;

            let a_score = (a.timestamp.timestamp() as f32 * recency_weight) + 
                         (a.relevance_score * relevance_weight);
            let b_score = (b.timestamp.timestamp() as f32 * recency_weight) + 
                         (b.relevance_score * relevance_weight);

            b_score.partial_cmp(&a_score).unwrap()
        });

        // Keep the most relevant conversations
        conversations.truncate(self.max_size);
        self.conversations = conversations.into_iter().collect();

        // Rebuild topic index
        self.rebuild_topic_index();
    }

    fn rebuild_topic_index(&mut self) {
        self.topic_index.clear();
        for (i, conv) in self.conversations.iter().enumerate() {
            for topic in &conv.topics {
                self.topic_index
                    .entry(topic.clone())
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }
    }

    pub fn get_context(&self, current_input: &str) -> String {
        let current_topics = self.extract_topics(current_input, "");
        let mut relevant_conversations: Vec<_> = self.conversations
            .iter()
            .enumerate()
            .map(|(i, conv)| {
                let topic_overlap = conv.topics
                    .iter()
                    .filter(|t| current_topics.contains(t))
                    .count();
                let recency_score = (self.conversations.len() - i) as f32;
                let relevance = (topic_overlap as f32 * 0.6) + (recency_score * 0.4);
                (conv, relevance)
            })
            .collect();

        // Sort by relevance
        relevant_conversations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Take top 10 most relevant conversations
        let context: Vec<String> = relevant_conversations
            .iter()
            .take(10)
            .map(|(conv, _)| {
                format!("User: {}\nAssistant: {}", conv.user_input, conv.ai_response)
            })
            .collect();

        context.join("\n\n")
    }

    pub fn conversation_count(&self) -> usize {
        self.conversations.len()
    }

    pub fn get_memory_stats(&self) -> String {
        let total_conversations = self.conversations.len();
        let total_topics: usize = self.topic_index.len();
        let avg_relevance: f32 = self.conversations
            .iter()
            .map(|c| c.relevance_score)
            .sum::<f32>() / total_conversations as f32;

        format!(
            "Memory Stats:\n\
             - Total Conversations: {}\n\
             - Unique Topics: {}\n\
             - Average Relevance: {:.2}\n\
             - Oldest Conversation: {}\n\
             - Most Recent: {}",
            total_conversations,
            total_topics,
            avg_relevance,
            self.conversations.front()
                .map(|c| c.timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "None".to_string()),
            self.conversations.back()
                .map(|c| c.timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "None".to_string())
        )
    }
}
