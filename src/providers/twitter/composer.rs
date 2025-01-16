use crate::personality::PersonalityProfile;
use crate::providers::twitter::twitbrain::Mention;
use crate::providers::deepseek::deepseek::DeepSeekProvider;
use crate::completion::CompletionProvider;
use anyhow::Result;

const MAX_TWEET_LENGTH: usize = 270;
const DEFAULT_EMOJI: &str = "💭";

pub struct TweetComposer;

impl TweetComposer {
    async fn get_deepseek_provider(profile: &PersonalityProfile) -> Result<DeepSeekProvider> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| anyhow::anyhow!("DEEPSEEK_API_KEY environment variable is not set. Please set it to use AI tweet generation."))?;
        
        // Get the base system message from the profile
        let mut system_parts = vec![profile.generate_system_prompt()];

        // Add tweet-specific instructions
        system_parts.push(format!("\nWhen tweeting, you should:\n\
            - Share insights from your expertise in {}\n\
            - Maintain your unique voice and personality traits\n\
            - Keep your communication style consistent\n\
            - Draw from your specific knowledge and experience\n\
            - Stay authentic to your character\n\n\
            Remember: You are {} - {}. Always tweet in character.", 
            profile.get_str("expertise").unwrap_or("your field"),
            profile.name,
            profile.get_str("description").unwrap_or("an expert in your field")
        ));

        // Add example tweets if available
        if let Some(examples) = profile.attributes.get("example_tweets") {
            if let Some(arr) = examples.as_array() {
                let example_list: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str())
                    .take(3)
                    .enumerate()
                    .map(|(i, t)| format!("{}. {}", i + 1, t))
                    .collect();
                if !example_list.is_empty() {
                    system_parts.push(format!("\nYour tweet style examples (maintain similar voice and approach):\n{}", 
                        example_list.join("\n")
                    ));
                }
            }
        }

        let system_message = system_parts.join("\n");

        DeepSeekProvider::new(api_key, system_message)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create DeepSeek provider: {}", e))
    }

    // Helper function to count approximate tokens (rough estimation)
    fn count_tokens(text: &str) -> usize {
        // Rough approximation: split on whitespace and punctuation
        text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .count()
    }

    pub async fn generate_auto_post_topic(profile: &PersonalityProfile) -> Result<String> {
        let mut prompt_parts = vec![
            format!("You are {}", profile.name),
            format!("Role: {}", profile.get_str("description").unwrap_or_default()),
            format!("Style: {}", profile.get_str("style").unwrap_or_default())
        ];
        
        // Add personality traits with more emphasis
        if let Some(traits) = profile.get_array("traits") {
            let trait_list: Vec<_> = traits.iter()
                .filter_map(|v| v.as_str())
                .collect();
            if !trait_list.is_empty() {
                prompt_parts.push(format!("Core personality traits: {}", trait_list.join(", ")));
            }
        }

        // Add expertise and interests with more weight
        if let Some(interests) = profile.get_array("interests") {
            let interest_list: Vec<_> = interests.iter()
                .filter_map(|v| v.as_str())
                .collect();
            if !interest_list.is_empty() {
                prompt_parts.push(format!("Primary areas of expertise: {}", interest_list.join(", ")));
            }
        }

        // Add communication style preferences
        if let Some(prefs) = profile.attributes.get("communication_preferences") {
            if let Some(obj) = prefs.as_object() {
                if let Some(style) = obj.get("primary_style") {
                    prompt_parts.push(format!("Communication approach: {}", style.as_str().unwrap_or_default()));
                }
                if let Some(complexity) = obj.get("complexity") {
                    prompt_parts.push(format!("Technical depth: {}", complexity.as_str().unwrap_or_default()));
                }
            }
        }

        // Add motto if available
        if let Some(motto) = profile.get_str("motto") {
            prompt_parts.push(format!("Your motto: \"{}\"", motto));
        }

        // Add example tweets with more context
        if let Some(examples) = profile.attributes.get("example_tweets") {
            if let Some(arr) = examples.as_array() {
                let example_list: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str())
                    .take(3) // Limit to 3 most relevant examples
                    .enumerate()
                    .map(|(i, t)| format!("{}. {}", i + 1, t))
                    .collect();
                if !example_list.is_empty() {
                    prompt_parts.push(format!("\nReference writing style (maintain similar tone and expertise level):\n{}", example_list.join("\n")));
                }
            }
        }

        prompt_parts.push("\nTask: Generate a topic for a tweet that:
1. Reflects your specific expertise and interests
2. Maintains your unique personality and communication style
3. Demonstrates your depth of knowledge in your field
4. Feels authentic to your character's background
5. Aligns with your typical discussion topics
6. Think of million random your though
7. Dont make Repeat topics again and again 

Topic:".to_string());

        let prompt = prompt_parts.join("\n\n");
        
        let provider = Self::get_deepseek_provider(profile).await?;
        let topic = provider.complete(&prompt).await?;
        
        // Clean up the topic
        let topic = topic.trim()
            .trim_start_matches("Topic:")
            .trim_start_matches("\"")
            .trim_end_matches("\"")
            .trim();
        
        Ok(topic.to_string())
    }

    #[inline]
    pub async fn generate_auto_tweet(profile: &PersonalityProfile) -> Result<String> {
        let topic = Self::generate_auto_post_topic(profile).await?;
        
        let mut prompt_parts = vec![
            format!("You are {} - {}", 
                profile.name,
                profile.get_str("description").unwrap_or_default()
            )
        ];

        // Add character's core identity
        prompt_parts.push(profile.generate_system_prompt());

        // Add tweet-specific instructions
        prompt_parts.push(format!("\nTask: Write a tweet about this topic : \"{}\"\n\nRequirements:\n\
            1. Write authentically as {} - maintain your unique voice\n\
            2. Draw from your expertise in {}\n\
            3. Use your characteristic communication style\n\
            4. Keep your personality traits consistent\n\
            5. Stay within Twitter's character limit at 260 character \n\
            6. Make it engaging and true to your character\n\n\
            Tweet:", 
            topic,
            profile.name,
            profile.get_str("expertise").unwrap_or("your field")
        ));

        let prompt = prompt_parts.join("\n\n");
        let provider = Self::get_deepseek_provider(profile).await?;
        let tweet = provider.complete(&prompt).await?;
        
        Ok(Self::truncate_content(tweet.trim()
            .trim_start_matches("Tweet:")
            .trim_start_matches("\"")
            .trim_end_matches("\"")
            .trim()
            .to_string()))
    }

    pub async fn generate_auto_reply(profile: &PersonalityProfile, original_tweet: &str) -> Result<String> {
        let deepseek = Self::get_deepseek_provider(profile).await?;
        let prompt = format!(
            "As {}, create a thoughtful reply to this tweet: '{}' \
             Maintain your unique voice while adding value to the conversation.",
            profile.name,
            original_tweet
        );
        let reply = deepseek.complete(&prompt).await?;
        Ok(Self::truncate_content(reply))
    }

    pub async fn generate_dm(profile: &PersonalityProfile, recipient: &str) -> Result<String> {
        let deepseek = Self::get_deepseek_provider(profile).await?;
        let prompt = format!(
            "As {}, write a professional direct message to @{}. \
             Keep it friendly yet professional, reflecting your personality.",
            profile.name,
            recipient
        );
        let dm = deepseek.complete(&prompt).await?;
        Ok(Self::truncate_content(dm))
    }

    pub async fn generate_mention_response(profile: &PersonalityProfile, mention: &Mention) -> Result<String> {
        let deepseek = Self::get_deepseek_provider(profile).await?;
        let prompt = format!(
            "As {}, respond to this mention: '{}' \
             Keep your response engaging and authentic to your character.",
            profile.name,
            mention.text
        );
        let response = deepseek.complete(&prompt).await?;
        Ok(Self::truncate_content(response))
    }

    fn truncate_content(content: String) -> String {
        content.chars().take(MAX_TWEET_LENGTH).collect()
    }
}
