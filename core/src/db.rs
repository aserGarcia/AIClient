use crate::{
    chat::{Chat, ChatMessage},
    directory,
};
use iced::widget::markdown;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct Database {
    pub needs_save: bool,
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let conn = Self::connect()?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            needs_save: false,
        })
    }

    pub fn connect() -> Result<Connection, rusqlite::Error> {
        let path = Self::get_db_path()?;
        let conn = Connection::open(path)?;

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chats (
                id BLOB PRIMARY KEY,
                title TEXT NOT NULL
            )",
            (),
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER,
                chat_id BLOB NOT NULL,
                content TEXT NOT NULL,
                is_reply INTEGER NOT NULL,
                PRIMARY KEY (id, chat_id),
                FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
            )",
            (),
        )?;

        Ok(conn)
    }

    pub fn get_db_path() -> Result<PathBuf, rusqlite::Error> {
        let dir = directory::config();

        std::fs::create_dir_all(&dir)
            .map_err(|e| rusqlite::Error::InvalidPath(e.to_string().into()))?;

        Ok(dir.join("convo.db"))
    }

    pub fn load_chats(&self) -> Result<Vec<Chat>, rusqlite::Error> {
        let binding = self.conn.lock().unwrap();
        let mut statement = match binding.prepare("SELECT id, title FROM chats") {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to prep load_chats: {}", e.to_string());
                return Err(e);
            }
        };

        let chats = match statement.query_map([], |row| {
            let chat_id: Uuid = Uuid::from_bytes(row.get(0)?);
            let messages = Self::load_messages(&binding, &chat_id);

            match messages {
                Ok(m) => {
                    info!("Retrieved {} messages for chat {}", m.len(), chat_id);
                    Ok(Chat {
                        id: chat_id,
                        title: row.get(1)?,
                        messages: m,
                    })
                }
                Err(e) => Err(e),
            }
        }) {
            Ok(c) => c,
            Err(e) => {
                error!("Error laoding chats: {}", e);
                return Err(e);
            }
        };

        Ok(chats.filter_map(|c| c.ok()).collect())
    }

    fn load_messages(
        conn: &Connection,
        chat_id: &Uuid,
    ) -> Result<Vec<ChatMessage>, rusqlite::Error> {
        let mut statement = match conn.prepare(
            "SELECT id, chat_id, content, is_reply FROM messages where chat_id = ? ORDER BY id",
        ) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to prep load_messages: {}", e);
                return Err(e);
            }
        };

        let messages = match statement.query_map([*chat_id.as_bytes()], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(2)?;
            Ok(ChatMessage {
                id: id as usize,
                chat_id: Uuid::from_bytes(row.get(1)?),
                content: content.clone(),
                markdown: markdown::Content::parse(content.as_str()),
                is_reply: row.get(3)?,
            })
        }) {
            Ok(c) => c,
            Err(e) => {
                error!(
                    "Error loading chat messages for chat {}",
                    chat_id.to_string()
                );
                return Err(e);
            }
        };

        Ok(messages.filter_map(|c| c.ok()).collect())
    }

    pub fn save_chat(&self, chat: &Chat) -> Result<(), rusqlite::Error> {
        let db = match self.conn.lock() {
            Ok(conn) => conn,
            Err(e) => {
                error!("Could not lock connection");
                return Err(rusqlite::Error::InvalidParameterName(e.to_string()));
            }
        };

        info!(
            "Saving chat {} with {} messages",
            chat.id,
            chat.messages.len()
        );
        db.execute(
            "INSERT OR REPLACE INTO chats (id, title) VALUES (?1, ?2)",
            params![*chat.id.as_bytes(), chat.title],
        )?;
        info!("Chat {} saved", chat.id);

        for msg in &chat.messages {
            db.execute(
                "INSERT OR REPLACE INTO messages (id, chat_id, content, is_reply) 
             VALUES (?1, ?2, ?3, ?4)",
                params![
                    msg.id as i64,
                    *msg.chat_id.as_bytes(),
                    msg.content,
                    msg.is_reply,
                ],
            )?;
        }
        debug!("All messages saved for chat {}", chat.id);
        Ok(())
    }

    pub fn delete_chat(&self, chat_id: &Uuid) -> Result<(), rusqlite::Error> {
        let db = match self.conn.lock() {
            Ok(conn) => conn,
            Err(e) => {
                error!("Could not lock connection");
                return Err(rusqlite::Error::InvalidParameterName(e.to_string()));
            }
        };
        if let Err(e) = db.execute(
            "DELETE FROM messages WHERE chat_id = ?",
            params![*chat_id.as_bytes()],
        ) {
            error!("Error deleting messages from chat {}: {}", chat_id, e);
            return Err(e);
        };

        if let Err(e) = db.execute(
            "DELETE FROM chats WHERE id = ?",
            params![*chat_id.as_bytes()],
        ) {
            error!("Error deleting chat {}", chat_id.to_string());
            return Err(e);
        };
        Ok(())
    }
}
