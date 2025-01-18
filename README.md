# Rust-Ai-Agent - Deepseek
My 1st project using Rust , Just check , maybe useful  😁
im noob, start from scratch , learning programming just copy and paste generate code from Ai 😆 😆 , im not dev , this is just test 

to start 

Get your Api key from deepseek 
https://platform.deepseek.com/

just only $2 for activated this API

and create .env in you root dir

# DeepSeek Configuration 
DEEPSEEK_API_KEY=

DEEPSEEK_BASE_URL=https://api.deepseek.com

DEEPSEEK_MODEL=deepseek-chat

DEEPSEEK_MAX_TOKENS=2048

DEEPSEEK_TEMPERATURE=0.7

and then 

cargo Run 

happy to chat with ur own deepseek 

# Example result 

![image](https://github.com/user-attachments/assets/5ccf5cf7-8570-4125-8dff-2669010ed5cb)


======================

# Rust AI Agent: Intelligent Conversational System 

## 🚀 Project Overview

### Vision
An advanced, modular AI agent built in Rust, designed to provide intelligent, context-aware, and dynamically adaptive conversational experiences.

## 🧠 Core Features

### 1. Dynamic Personality System
- **Modular Character Profiles**
  - JSON-based personality configuration
  - Rich emotional expression capabilities
  - Customizable communication styles

### 2. Intelligent Conversation Management
- **Persistent Memory Storage**
  - SQLite-powered conversation tracking
  - Context retention and learning
  - Dynamic knowledge expansion

### 3. Emotional Intelligence
- **Emoji and Emote Support**
  - Context-specific emotional expressions
  - Adaptive communication strategies
  - Enhanced interaction depth

## 🔧 Technical Architecture

### Language and Technologies
- **Primary Language**: Rust
- **Database**: SQLite (rusqlite)
- **Serialization**: Serde
- **Character Management**: JSON-based configuration

### Key Components
- Personality Loader
- Conversation Tracker
- Emotion Expression Engine
- Knowledge Base Manager

## 👥 Included Characters
1. **Zara "CodeWizard" Chen**
   - Programming-focused personality
   - Technical humor specialist

2. **Dr. Rissa**
   - Neuroscience researcher
   - Analytical communication style

3. **Joey**
   - Culinary science expert
   - Experimental communication approach

4. **Alex Chen**
   - Startup founder
   - Innovation-driven communicator


## 🤝 Contribution
fell free  

## 💡 Getting Started
```bash
# Clone the repository
git clone https://github.com/ZoeyX-FD/Rust-Ai-Agent---Deepseek.git

# Build the project
cargo build

# Run the AI agent
cargo run

## 🎭 Loading Characters ( Im Inspired by ElizaOS ) The best role Model

### Character Selection Methods

#### 1. Interactive Character Selection
When you run the AI agent, you'll see a prompt to choose a character:
Available Characters:

Type 'coding_ninja' for Zara "CodeWizard" Chen
Type 'academic_researcher' for Dr. Rissa
Type 'masterchef_scientist' for Joey
Type 'startup_founder' for Alex Chen


#### 2. Direct Filename Loading
You can load any character by typing its filename:
```bash
# Load a character directly by filename
masterchef_scientist.json

3. Programmatic Character Loading
In your Rust code, you can load characters programmatically:

// Create a new character dynamically

let custom_character = PersonalityProfile {
    name: "Custom Character".to_string(),

// Add more custom configuration
};

{
    "name": "Your Character Name",
    "bio": { ... },
    "traits": { ... },
    "emotions": {
        "expressions": {
            "emotion_name": {
                "emojis": ["😄", "🚀"],
                "emotes": ["*does something*"]
            }
        }
    }
}

Best Practices
Keep character files in characters/ directory
Use meaningful, descriptive filenames
Maintain consistent JSON structure
Experiment with different personality traits

# Web Crawler Setup

For web crawling functionality, you need to have a WebDriver running. The easiest way is to use ChromeDriver:

1. Install ChromeDriver:
   ```bash
   # For Ubuntu/Debian
   sudo apt install chromium-chromedriver
   
   # For macOS
   brew install chromedriver
   
   # For Windows, download from:
   # https://chromedriver.chromium.org/downloads
   ```

2. Start ChromeDriver:
   ```bash
   chromedriver --port=4444
   ```

3. Run the program with web crawler enabled:
   ```bash
   cargo run -- --crawler
   ```

You can also enable the web crawler by setting the environment variable:
```bash
export ENABLE_CRAWLER=true
cargo run
```
