use tokio_rusqlite::Connection;
use std::path::Path;
use log::{info, error};
use thiserror::Error;
use std::sync::Arc;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] tokio_rusqlite::Error),
    #[error("Database connection error: {0}")]
    Connection(String),
}

#[derive(Clone)]
pub struct Database {
    conn: Arc<Connection>,
}

impl Database {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)
            .await
            .map_err(|e| DatabaseError::Connection(e.to_string()))?;
        
        let db = Self { conn: Arc::new(conn) };
        db.initialize().await?;
        Ok(db)
    }

    async fn initialize(&self) -> Result<(), DatabaseError> {
        // Create tables if they don't exist
        self.conn.call(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS conversations (
                    id INTEGER PRIMARY KEY,
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                    user_input TEXT NOT NULL,
                    ai_response TEXT NOT NULL,
                    personality TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS knowledge_base (
                    id INTEGER PRIMARY KEY,
                    key TEXT UNIQUE NOT NULL,
                    value TEXT NOT NULL,
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS document_insights (
                    id INTEGER PRIMARY KEY,
                    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                    document_path TEXT NOT NULL,
                    insight_text TEXT NOT NULL,
                    relevance REAL NOT NULL,
                    insight_type TEXT NOT NULL
                );"
            )
        })
        .await?;

        info!("Database initialized successfully");
        Ok(())
    }

    pub async fn save_conversation(
        &self,
        user_input: String,
        ai_response: String,
        personality: String,
    ) -> Result<(), DatabaseError> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO conversations (user_input, ai_response, personality) VALUES (?1, ?2, ?3)",
                    [&user_input, &ai_response, &personality],
                )
            })
            .await?;
        
        Ok(())
    }

    pub async fn save_knowledge(
        &self,
        key: String,
        value: String,
    ) -> Result<(), DatabaseError> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR REPLACE INTO knowledge_base (key, value) VALUES (?1, ?2)",
                    [&key, &value],
                )
            })
            .await?;
        
        Ok(())
    }

    pub async fn get_recent_conversations(&self, limit: i64) -> Result<Vec<(String, String, String, String)>, DatabaseError> {
        let result = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT timestamp, user_input, ai_response, personality 
                     FROM conversations 
                     ORDER BY timestamp DESC 
                     LIMIT ?"
                )?;
                
                let rows = stmt.query_map([limit], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?;

                let mut conversations = Vec::new();
                for row in rows {
                    conversations.push(row?);
                }
                
                Ok(conversations)
            })
            .await?;
            
        Ok(result)
    }

    pub async fn get_knowledge(&self, key: String) -> Result<Option<String>, DatabaseError> {
        let result = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT value FROM knowledge_base WHERE key = ?")?;
                let mut rows = stmt.query([&key])?;
                
                if let Some(row) = rows.next()? {
                    Ok(Some(row.get::<_, String>(0)?))
                } else {
                    Ok(None)
                }
            })
            .await?;
            
        Ok(result)
    }

    pub async fn save_document_insight(
        &self,
        document_path: String,
        insight_text: String,
        relevance: f32,
        insight_type: String,
    ) -> Result<(), DatabaseError> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO document_insights (document_path, insight_text, relevance, insight_type) 
                     VALUES (?1, ?2, ?3, ?4)",
                    [&document_path, &insight_text, &relevance.to_string(), &insight_type],
                )
            })
            .await?;
        
        Ok(())
    }

    pub async fn get_document_insights(
        &self,
        document_path: String,
    ) -> Result<Vec<(String, String, f32, String)>, DatabaseError> {
        let result = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT timestamp, insight_text, relevance, insight_type 
                     FROM document_insights 
                     WHERE document_path = ?
                     ORDER BY timestamp DESC"
                )?;
                
                let rows = stmt.query_map([&document_path], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?.parse::<f32>().unwrap_or(0.0),
                        row.get::<_, String>(3)?,
                    ))
                })?;

                let mut insights = Vec::new();
                for row in rows {
                    insights.push(row?);
                }
                
                Ok(insights)
            })
            .await?;
            
        Ok(result)
    }

    pub async fn search_document_insights(
        &self,
        query: &str,
    ) -> Result<Vec<(String, String, f32)>, DatabaseError> {
        let query = query.to_string();
        let result = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT document_path, insight_text, relevance 
                     FROM document_insights 
                     WHERE insight_text LIKE ?1 
                     ORDER BY relevance DESC"
                )?;
                
                let search_pattern = format!("%{}%", query);
                let rows = stmt.query_map([search_pattern], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?.parse::<f32>().unwrap_or(0.0),
                    ))
                })?;

                let mut insights = Vec::new();
                for row in rows {
                    insights.push(row?);
                }
                
                Ok(insights)
            })
            .await?;
            
        Ok(result)
    }

    pub async fn get_all_document_insights(&self) -> Result<Vec<(String, String, f32, String)>, DatabaseError> {
        let result = self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT document_path, insight_text, relevance, insight_type 
                     FROM document_insights 
                     ORDER BY relevance DESC"
                )?;
                
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?.parse::<f32>().unwrap_or(0.0),
                        row.get::<_, String>(3)?,
                    ))
                })?;

                let mut insights = Vec::new();
                for row in rows {
                    insights.push(row?);
                }
                
                Ok(insights)
            })
            .await?;
            
        Ok(result)
    }
}
