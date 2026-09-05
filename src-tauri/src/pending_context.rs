use crate::passport::CaptureContext;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const PENDING_CONTEXT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS pending_browser_context (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    browser_name TEXT,
    page_title TEXT,
    page_url TEXT,
    link_text TEXT,
    context_text TEXT,
    context_source TEXT,
    captured_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_pending_browser_context_time
ON pending_browser_context(captured_at DESC, id DESC);
"#;

pub fn initialize_database(path: &Path) -> Result<(), String> {
    crate::passport::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(PENDING_CONTEXT_SCHEMA)
        .map_err(|error| error.to_string())
}

pub fn record(path: &Path, context: &CaptureContext) -> Result<(), String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute(
            r#"
            INSERT INTO pending_browser_context (
                browser_name, page_title, page_url, link_text, context_text, context_source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                context.browser_name,
                context.page_title,
                context.page_url,
                context.link_text,
                context.context_text,
                context.context_source,
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "DELETE FROM pending_browser_context WHERE id NOT IN (SELECT id FROM pending_browser_context ORDER BY id DESC LIMIT 20)",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn recent(path: &Path) -> Result<Option<CaptureContext>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .query_row(
            r#"
            SELECT browser_name, page_title, page_url, link_text, context_text, context_source
            FROM pending_browser_context
            WHERE captured_at >= datetime('now', '-10 minutes')
            ORDER BY id DESC
            LIMIT 1
            "#,
            [],
            |row| {
                Ok(CaptureContext {
                    browser_name: row.get(0)?,
                    page_title: row.get(1)?,
                    page_url: row.get(2)?,
                    link_text: row.get(3)?,
                    context_text: row.get(4)?,
                    context_source: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    #[test]
    fn keeps_recent_context_bounded() {
        let path = env::temp_dir().join("originkeep-pending-context-test.db");
        fs::remove_file(&path).ok();
        for index in 0..25 {
            record(
                &path,
                &CaptureContext {
                    page_title: Some(format!("page-{index}")),
                    context_source: Some("safari-fallback".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        let connection = Connection::open(&path).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM pending_browser_context", [], |row| row.get(0))
            .unwrap();
        fs::remove_file(&path).ok();
        assert_eq!(count, 20);
    }
}
