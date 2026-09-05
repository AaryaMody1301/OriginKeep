use crate::model::{DownloadCapture, DownloadRecord, IngestResult};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS downloads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    capture_key TEXT NOT NULL UNIQUE,
    browser_download_id INTEGER NOT NULL,
    original_url TEXT NOT NULL,
    final_url TEXT,
    referrer TEXT,
    local_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime_type TEXT,
    bytes INTEGER,
    started_at TEXT,
    completed_at TEXT,
    sha256 TEXT,
    status TEXT NOT NULL DEFAULT 'SOURCE_UNKNOWN',
    browser_state TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_downloads_sha256 ON downloads(sha256);
CREATE INDEX IF NOT EXISTS idx_downloads_final_url ON downloads(final_url);
CREATE INDEX IF NOT EXISTS idx_downloads_file_name ON downloads(file_name);
"#;

pub fn default_database_path() -> Result<PathBuf, String> {
    let base = if cfg!(target_os = "windows") {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if let Some(path) = env::var_os("XDG_DATA_HOME") {
        Some(PathBuf::from(path))
    } else {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
    }
    .ok_or_else(|| "Could not determine a local application-data directory".to_string())?;

    Ok(base.join("OriginKeep").join("originkeep.db"))
}

pub fn initialize_database(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())
}

fn initialize_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(SCHEMA)
}

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn ingest_capture(path: &Path, capture: &DownloadCapture) -> Result<IngestResult, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    ingest_capture_with_connection(&connection, capture).map_err(|error| error.to_string())
}

fn ingest_capture_with_connection(
    connection: &Connection,
    capture: &DownloadCapture,
) -> rusqlite::Result<IngestResult> {
    initialize_connection(connection)?;

    let local_path = Path::new(&capture.local_path);
    let sha256 = if capture.state == "complete" && local_path.is_file() {
        sha256_file(local_path).ok()
    } else {
        None
    };

    let status = if capture.state == "complete" && !local_path.is_file() {
        "LOCAL_MISSING"
    } else {
        "SOURCE_UNKNOWN"
    };

    connection.execute(
        r#"
        INSERT INTO downloads (
            capture_key, browser_download_id, original_url, final_url, referrer,
            local_path, file_name, mime_type, bytes, started_at, completed_at,
            sha256, status, browser_state
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(capture_key) DO UPDATE SET
            original_url = excluded.original_url,
            final_url = excluded.final_url,
            referrer = excluded.referrer,
            local_path = excluded.local_path,
            file_name = excluded.file_name,
            mime_type = excluded.mime_type,
            bytes = excluded.bytes,
            started_at = excluded.started_at,
            completed_at = excluded.completed_at,
            sha256 = COALESCE(excluded.sha256, downloads.sha256),
            status = excluded.status,
            browser_state = excluded.browser_state,
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![
            capture.capture_key,
            capture.browser_download_id,
            capture.original_url,
            capture.final_url,
            capture.referrer,
            capture.local_path,
            capture.file_name,
            capture.mime_type,
            capture.bytes,
            capture.started_at,
            capture.completed_at,
            sha256,
            status,
            capture.state,
        ],
    )?;

    let id: i64 = connection.query_row(
        "SELECT id FROM downloads WHERE capture_key = ?1",
        [&capture.capture_key],
        |row| row.get(0),
    )?;

    Ok(IngestResult {
        ok: true,
        id,
        sha256,
        status: status.to_string(),
    })
}

pub fn list_downloads(path: &Path, query: Option<&str>) -> Result<Vec<DownloadRecord>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    list_downloads_with_connection(&connection, query).map_err(|error| error.to_string())
}

fn list_downloads_with_connection(
    connection: &Connection,
    query: Option<&str>,
) -> rusqlite::Result<Vec<DownloadRecord>> {
    let normalized = query.map(str::trim).filter(|value| !value.is_empty());
    let search = normalized.map(|value| format!("%{value}%"));

    let sql = if search.is_some() {
        r#"
        SELECT id, capture_key, original_url, final_url, referrer, local_path,
               file_name, mime_type, bytes, started_at, completed_at, sha256,
               status, updated_at
        FROM downloads
        WHERE file_name LIKE ?1
           OR original_url LIKE ?1
           OR COALESCE(final_url, '') LIKE ?1
           OR COALESCE(referrer, '') LIKE ?1
           OR COALESCE(sha256, '') LIKE ?1
        ORDER BY updated_at DESC, id DESC
        "#
    } else {
        r#"
        SELECT id, capture_key, original_url, final_url, referrer, local_path,
               file_name, mime_type, bytes, started_at, completed_at, sha256,
               status, updated_at
        FROM downloads
        ORDER BY updated_at DESC, id DESC
        "#
    };

    let mut statement = connection.prepare(sql)?;
    let mapper = |row: &rusqlite::Row<'_>| {
        Ok(DownloadRecord {
            id: row.get(0)?,
            capture_key: row.get(1)?,
            original_url: row.get(2)?,
            final_url: row.get(3)?,
            referrer: row.get(4)?,
            local_path: row.get(5)?,
            file_name: row.get(6)?,
            mime_type: row.get(7)?,
            bytes: row.get(8)?,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            sha256: row.get(11)?,
            status: row.get(12)?,
            updated_at: row.get(13)?,
        })
    };

    if let Some(search) = search {
        statement.query_map([search], mapper)?.collect()
    } else {
        statement.query_map([], mapper)?.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn capture(path: &Path) -> DownloadCapture {
        DownloadCapture {
            capture_key: "extension:42:2026-09-04T12:00:00Z".into(),
            browser_download_id: 42,
            original_url: "https://example.com/report.pdf".into(),
            final_url: Some("https://cdn.example.com/report.pdf".into()),
            referrer: Some("https://example.com/reports".into()),
            local_path: path.display().to_string(),
            file_name: "report.pdf".into(),
            mime_type: Some("application/pdf".into()),
            bytes: Some(11),
            started_at: Some("2026-09-04T12:00:00Z".into()),
            completed_at: Some("2026-09-04T12:00:01Z".into()),
            state: "complete".into(),
        }
    }

    #[test]
    fn hashes_files_deterministically() {
        let path = env::temp_dir().join("originkeep-sha-test.txt");
        fs::write(&path, b"hello world").unwrap();
        let hash = sha256_file(&path).unwrap();
        fs::remove_file(path).ok();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn capture_upsert_is_idempotent_and_searchable() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file_path = env::temp_dir().join(format!("originkeep-{unique}.pdf"));
        fs::write(&file_path, b"hello world").unwrap();
        let item = capture(&file_path);

        let first = ingest_capture_with_connection(&connection, &item).unwrap();
        let second = ingest_capture_with_connection(&connection, &item).unwrap();
        let rows = list_downloads_with_connection(&connection, Some("example.com")).unwrap();
        fs::remove_file(file_path).ok();

        assert_eq!(first.id, second.id);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].sha256.is_some());
    }
}
