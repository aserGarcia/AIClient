use crate::{
    chat::{Chat, ChatMessage},
    directory,
};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL
            )",
            (),
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY,
                chat_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                is_reply INTEGER NOT NULL,
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

        Ok(dir.join("chats.db"))
    }

    pub fn load_chats(&self) -> Vec<Chat> {
        let binding = self.conn.lock().unwrap();
        let mut statement = match binding.prepare("SELECT id, title FROM chats ORDER BY id") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to prep load_chats: {}", e);
                return Vec::new();
            }
        };

        // TODO: Handle error more gracefully
        let chats = match statement.query_map([], |row| {
            let chat_id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let messages = Self::load_messages(&binding, chat_id);
            println!("Retrieved {} messages for chat {}", messages.len(), chat_id);
            Ok(Chat {
                id: chat_id as usize,
                title,
                messages,
            })
        }) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error laoding chat: {}", e);
                return Vec::new();
            }
        };

        // TODO: Dumps the error chats?
        chats.filter_map(|c| c.ok()).collect()
    }

    fn load_messages(conn: &Connection, chat_id: i64) -> Vec<ChatMessage> {
        let mut statement = match conn.prepare(
            "SELECT id, chat_id, content, is_reply FROM messages where chat_id = ? ORDER BY id",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to prep load_messages: {}", e);
                return Vec::new();
            }
        };

        let messages = match statement.query_map([chat_id], |row| {
            let id: i64 = row.get(0)?;
            let chat_id: i64 = row.get(1)?;
            Ok(ChatMessage {
                id: id as usize,
                chat_id: chat_id as usize,
                content: row.get(2)?,
                is_reply: row.get(3)?,
            })
        }) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        // TODO: Drops the error chats?
        messages.filter_map(|c| c.ok()).collect()
    }

    pub fn save_chat(&self, chat: &Chat) -> Result<(), rusqlite::Error> {
        // TODO: Handle error case from lock
        let db = self.conn.lock().unwrap();

        println!(
            "Saving chat {} with {} messages",
            chat.id,
            chat.messages.len()
        );
        db.execute(
            "INSERT OR REPLACE INTO chats (id, title) VALUES (?1, ?2)",
            params![chat.id as i64, chat.title],
        )?;
        println!("Chat saved");

        for msg in &chat.messages {
            db.execute(
                "INSERT OR REPLACE INTO messages (id, chat_id, content, is_reply) 
             VALUES (?1, ?2, ?3, ?4)",
                params![msg.id as i64, msg.chat_id as i64, msg.content, msg.is_reply,],
            )?;
            println!("Message saved")
        }
        println!("All messages saved for chat {}", chat.id);
        Ok(())
    }

    pub fn delete_chat(&self, chat_id: usize) -> Result<(), rusqlite::Error> {
        let db = self.conn.lock().unwrap();
        if let Err(e) = db.execute(
            "DELETE FROM messages WHERE chat_id = ?",
            params![chat_id as i64],
        ) {
            eprintln!("Error deleting messages from chat {}: {}", chat_id, e);
        };

        db.execute("DELETE FROM chats WHERE id = ?", params![chat_id as i64])?;
        Ok(())
    }
}
