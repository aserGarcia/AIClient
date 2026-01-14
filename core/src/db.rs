use crate::directory;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Database {
    pub conn: Arc<Mutex<Connection>>,
    pub needs_save: bool,
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
}
