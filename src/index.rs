use crate::error::{PensieveError, Result};
use rusqlite::params;
use std::path::Path;
use std::sync::Mutex;

// Short words that add noise to FTS5 keyword searches
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "been", "but", "by", "do", "for",
    "from", "had", "has", "have", "he", "her", "his", "how", "i", "if", "in",
    "is", "it", "its", "me", "my", "no", "not", "of", "on", "or", "our", "out",
    "she", "so", "than", "that", "the", "their", "them", "then", "there", "they",
    "this", "to", "up", "us", "was", "we", "what", "when", "where", "which",
    "who", "will", "with", "you", "your",
];

/// Build an FTS5 query from a natural language string.
/// Single words are passed through. Multi-word queries use OR semantics
/// with stopwords removed so any meaningful term can match.
fn build_fts_query(query: &str) -> String {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|t| !t.is_empty() && t.len() > 1 && !STOPWORDS.contains(&t.as_str()))
        .collect();

    if terms.is_empty() {
        return query.trim().to_string();
    }
    terms.join(" OR ")
}

pub struct Index {
    conn: Mutex<rusqlite::Connection>,
}

fn f32_slice_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

impl Index {
    pub fn open(memory_dir: &Path) -> Result<Self> {
        // Register sqlite-vec extension before opening connection
        #[allow(unsafe_code)]
        unsafe {
            #[allow(clippy::missing_transmute_annotations)]
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        let db_path = memory_dir.join("index.sqlite");
        let conn = rusqlite::Connection::open(db_path)?;

        conn.execute_batch(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                memory_id, title, content, project, tags
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec USING vec0(
                memory_id TEXT PRIMARY KEY,
                embedding float[384]
            );",
        )?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn upsert(
        &self,
        memory_id: &str,
        title: &str,
        content: &str,
        project: Option<&str>,
        tags: &[String],
        embedding: Option<&[f32]>,
    ) -> Result<()> {
        let conn =
            self.conn.lock().map_err(|e| PensieveError::Config(format!("lock poisoned: {e}")))?;

        // Delete existing FTS entry
        conn.execute("DELETE FROM memory_fts WHERE memory_id = ?1", [memory_id])?;

        // Insert into FTS5
        let tags_str = tags.join(", ");
        let project_str = project.unwrap_or("");
        conn.execute(
            "INSERT INTO memory_fts (memory_id, title, content, project, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![memory_id, title, content, project_str, tags_str],
        )?;

        // Insert into vec0 if embedding provided
        if let Some(emb) = embedding {
            conn.execute("DELETE FROM memory_vec WHERE memory_id = ?1", [memory_id])?;
            let blob = f32_slice_to_bytes(emb);
            conn.execute(
                "INSERT INTO memory_vec (memory_id, embedding) VALUES (?1, ?2)",
                params![memory_id, blob],
            )?;
        }

        Ok(())
    }

    pub fn delete(&self, memory_id: &str) -> Result<()> {
        let conn =
            self.conn.lock().map_err(|e| PensieveError::Config(format!("lock poisoned: {e}")))?;

        conn.execute("DELETE FROM memory_fts WHERE memory_id = ?1", [memory_id])?;
        conn.execute("DELETE FROM memory_vec WHERE memory_id = ?1", [memory_id])?;

        Ok(())
    }

    pub fn recall_keyword(&self, query: &str, limit: usize) -> Result<Vec<(String, f64)>> {
        let conn =
            self.conn.lock().map_err(|e| PensieveError::Config(format!("lock poisoned: {e}")))?;

        let fts_query = build_fts_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }

        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut stmt = conn.prepare(
            "SELECT memory_id, bm25(memory_fts) AS score FROM memory_fts WHERE memory_fts MATCH ?1 ORDER BY score LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![fts_query, limit_i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .filter_map(std::result::Result::ok)
            .collect();

        Ok(results)
    }

    pub fn recall_vector(&self, embedding: &[f32], limit: usize) -> Result<Vec<(String, f64)>> {
        let conn =
            self.conn.lock().map_err(|e| PensieveError::Config(format!("lock poisoned: {e}")))?;

        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let blob = f32_slice_to_bytes(embedding);
        let mut stmt = conn.prepare(
            "SELECT memory_id, distance FROM memory_vec WHERE embedding MATCH ?1 ORDER BY distance LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![blob, limit_i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?
            .filter_map(std::result::Result::ok)
            .collect();

        Ok(results)
    }

    pub fn clear(&self) -> Result<()> {

        let conn =
            self.conn.lock().map_err(|e| PensieveError::Config(format!("lock poisoned: {e}")))?;

        conn.execute("DELETE FROM memory_fts", [])?;
        conn.execute("DELETE FROM memory_vec", [])?;
        Ok(())
    }
}
