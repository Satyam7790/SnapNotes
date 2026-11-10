use once_cell::sync::Lazy;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::sync::Mutex;

pub static DB: Lazy<Mutex<Store>> = Lazy::new(|| Mutex::new(Store { conn: None }));
pub struct Store { conn: Option<Connection> }
#[derive(Serialize, Clone)]
pub struct Workspace { pub id: String, pub title: String, pub document: serde_json::Value, pub created_at: String, pub updated_at: String, pub preview: String }
impl Store {
    pub fn open(&mut self, path: std::path::PathBuf) -> Result<(), rusqlite::Error> {
        let c = Connection::open(path)?;
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS workspaces(id TEXT PRIMARY KEY,title TEXT NOT NULL,document TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);")?;
        self.conn = Some(c); Ok(())
    }
    fn conn(&self) -> &Connection { self.conn.as_ref().expect("database initialized") }
    fn row(row: &rusqlite::Row<'_>) -> Result<Workspace, rusqlite::Error> {
        let raw: String = row.get(2)?; let document: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}));
        let preview = collect_text(&document);
        Ok(Workspace { id:row.get(0)?,title:row.get(1)?,document,created_at:row.get(3)?,updated_at:row.get(4)?,preview })
    }
    pub fn list(&self, query: Option<&str>) -> Result<Vec<Workspace>, rusqlite::Error> {
        let mut stmt = self.conn().prepare("SELECT id,title,document,created_at,updated_at FROM workspaces WHERE (?1 IS NULL OR title LIKE '%'||?1||'%' OR document LIKE '%'||?1||'%') ORDER BY updated_at DESC")?;
        let rows = stmt.query_map(params![query], Self::row)?; rows.collect()
    }
    pub fn get(&self, id: &str) -> Result<Option<Workspace>, rusqlite::Error> {
        self.conn().query_row("SELECT id,title,document,created_at,updated_at FROM workspaces WHERE id=?1", [id], Self::row).optional()
    }
    pub fn save(&self, id: &str, title: &str, doc: &serde_json::Value) -> Result<Workspace, rusqlite::Error> {
        let now = chrono::Utc::now().to_rfc3339(); let json = serde_json::to_string(doc).unwrap_or_else(|_| "{\"type\":\"doc\",\"content\":[{\"type\":\"paragraph\"}]}".into());
        self.conn().execute("INSERT INTO workspaces(id,title,document,created_at,updated_at) VALUES(?1,?2,?3,?4,?4) ON CONFLICT(id) DO UPDATE SET title=excluded.title,document=excluded.document,updated_at=excluded.updated_at",params![id,title,json,now])?;
        self.get(id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
    }
    pub fn delete(&self, id:&str)->Result<(),rusqlite::Error>{self.conn().execute("DELETE FROM workspaces WHERE id=?1",[id])?;Ok(())}
}
use rusqlite::OptionalExtension;
fn collect_text(value:&serde_json::Value)->String { let mut out=String::new(); if let Some(s)=value.get("text").and_then(|x|x.as_str()){out.push_str(s);out.push(' ')} if let Some(a)=value.get("content").and_then(|x|x.as_array()){for v in a{out.push_str(&collect_text(v));}} out }
