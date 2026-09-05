use crate::model::{DownloadCapture, DownloadRecord, IngestResult, VerificationSummary};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
};
use url::Url;

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
    source_identity TEXT,
    version_number INTEGER,
    duplicate_of_id INTEGER,
    local_state TEXT NOT NULL DEFAULT 'PRESENT',
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
    connection.execute_batch(SCHEMA)?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 2 {
        migrate_to_phase2(connection)?;
    }
    Ok(())
}

fn migrate_to_phase2(connection: &Connection) -> rusqlite::Result<()> {
    add_column_if_missing(connection, "source_identity", "TEXT")?;
    add_column_if_missing(connection, "version_number", "INTEGER")?;
    add_column_if_missing(connection, "duplicate_of_id", "INTEGER")?;
    add_column_if_missing(connection, "local_state", "TEXT NOT NULL DEFAULT 'PRESENT'")?;

    connection.execute_batch(
        r#"
        CREATE INDEX IF NOT EXISTS idx_downloads_source_identity ON downloads(source_identity);
        CREATE INDEX IF NOT EXISTS idx_downloads_duplicate_of_id ON downloads(duplicate_of_id);
        UPDATE downloads SET local_state = 'LOCAL_MISSING' WHERE status = 'LOCAL_MISSING';
        UPDATE downloads SET status = 'SOURCE_UNKNOWN' WHERE status = 'LOCAL_MISSING';
        "#,
    )?;

    backfill_phase2_metadata(connection)?;
    connection.execute_batch("PRAGMA user_version = 2;")?;
    Ok(())
}

fn add_column_if_missing(
    connection: &Connection,
    name: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    if !column_exists(connection, name)? {
        connection.execute_batch(&format!(
            "ALTER TABLE downloads ADD COLUMN {name} {definition};"
        ))?;
    }
    Ok(())
}

fn column_exists(connection: &Connection, name: &str) -> rusqlite::Result<bool> {
    let mut statement = connection.prepare("PRAGMA table_info(downloads)")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let column_name: String = row.get(1)?;
        if column_name == name {
            return Ok(true);
        }
    }
    Ok(false)
}

fn backfill_phase2_metadata(connection: &Connection) -> rusqlite::Result<()> {
    let records = {
        let mut statement = connection
            .prepare("SELECT id, original_url, final_url, sha256 FROM downloads ORDER BY id ASC")?;
        let records = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        records
    };

    for (id, original_url, final_url, sha256) in records {
        let source_identity = source_identity(&original_url, final_url.as_deref());
        connection.execute(
            "UPDATE downloads SET source_identity = ?1 WHERE id = ?2",
            params![source_identity, id],
        )?;

        if let Some(hash) = sha256 {
            assign_version_metadata(connection, id, &hash)?;
        }
    }

    Ok(())
}

pub fn canonicalize_source_url(value: &str) -> Option<String> {
    let mut parsed = Url::parse(value).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }

    parsed.set_fragment(None);
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    Some(parsed.to_string())
}

fn source_identity(original_url: &str, final_url: Option<&str>) -> Option<String> {
    canonicalize_source_url(original_url).or_else(|| final_url.and_then(canonicalize_source_url))
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
    let local_state = if capture.state == "complete" && !local_path.is_file() {
        "LOCAL_MISSING"
    } else {
        "PRESENT"
    };
    let identity = source_identity(&capture.original_url, capture.final_url.as_deref());

    connection.execute(
        r#"
        INSERT INTO downloads (
            capture_key, browser_download_id, original_url, final_url, referrer,
            local_path, file_name, mime_type, bytes, started_at, completed_at,
            sha256, status, browser_state, source_identity, local_state
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'SOURCE_UNKNOWN', ?13, ?14, ?15)
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
            browser_state = excluded.browser_state,
            source_identity = COALESCE(excluded.source_identity, downloads.source_identity),
            local_state = excluded.local_state,
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
            capture.state,
            identity,
            local_state,
        ],
    )?;

    let id: i64 = connection.query_row(
        "SELECT id FROM downloads WHERE capture_key = ?1",
        [&capture.capture_key],
        |row| row.get(0),
    )?;

    let effective_hash: Option<String> =
        connection.query_row("SELECT sha256 FROM downloads WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;

    if let Some(hash) = effective_hash.as_deref() {
        assign_version_metadata(connection, id, hash)?;
    }

    connection.query_row(
        r#"
        SELECT id, sha256, status, source_identity, version_number, duplicate_of_id, local_state
        FROM downloads WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(IngestResult {
                ok: true,
                id: row.get(0)?,
                sha256: row.get(1)?,
                status: row.get(2)?,
                source_identity: row.get(3)?,
                version_number: row.get(4)?,
                duplicate_of_id: row.get(5)?,
                local_state: row.get(6)?,
            })
        },
    )
}

fn assign_version_metadata(connection: &Connection, id: i64, hash: &str) -> rusqlite::Result<()> {
    let identity: Option<String> = connection.query_row(
        "SELECT source_identity FROM downloads WHERE id = ?1",
        [id],
        |row| row.get(0),
    )?;

    let duplicate_of_id: Option<i64> = connection
        .query_row(
            "SELECT id FROM downloads WHERE sha256 = ?1 AND id < ?2 ORDER BY id ASC LIMIT 1",
            params![hash, id],
            |row| row.get(0),
        )
        .optional()?;

    let version_number = if let Some(source) = identity.as_deref() {
        let existing_version: Option<i64> = connection
            .query_row(
                r#"
                SELECT version_number FROM downloads
                WHERE source_identity = ?1 AND sha256 = ?2 AND version_number IS NOT NULL
                ORDER BY id ASC LIMIT 1
                "#,
                params![source, hash],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(version) = existing_version {
            Some(version)
        } else {
            let max_version: Option<i64> = connection.query_row(
                "SELECT MAX(version_number) FROM downloads WHERE source_identity = ?1",
                [source],
                |row| row.get(0),
            )?;
            Some(max_version.unwrap_or(0) + 1)
        }
    } else {
        None
    };

    connection.execute(
        "UPDATE downloads SET version_number = ?1, duplicate_of_id = ?2 WHERE id = ?3",
        params![version_number, duplicate_of_id, id],
    )?;

    if let (Some(source), Some(version)) = (identity.as_deref(), version_number) {
        connection.execute(
            r#"
            UPDATE downloads
            SET status = 'SUPERSEDED'
            WHERE source_identity = ?1
              AND duplicate_of_id IS NULL
              AND version_number IS NOT NULL
              AND version_number < ?2
            "#,
            params![source, version],
        )?;
    }

    let max_family_version = if let Some(source) = identity.as_deref() {
        connection.query_row(
            "SELECT MAX(version_number) FROM downloads WHERE source_identity = ?1",
            [source],
            |row| row.get::<_, Option<i64>>(0),
        )?
    } else {
        None
    };

    let status = if duplicate_of_id.is_some() {
        "DUPLICATE"
    } else if version_number.is_some()
        && max_family_version.is_some()
        && version_number < max_family_version
    {
        "SUPERSEDED"
    } else {
        "SOURCE_UNKNOWN"
    };

    connection.execute(
        "UPDATE downloads SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
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
    initialize_connection(connection)?;
    let normalized = query.map(str::trim).filter(|value| !value.is_empty());
    let search = normalized.map(|value| format!("%{value}%"));

    let sql = if search.is_some() {
        r#"
        SELECT id, capture_key, original_url, final_url, referrer, local_path,
               file_name, mime_type, bytes, started_at, completed_at, sha256,
               status, source_identity, version_number, duplicate_of_id, local_state, updated_at
        FROM downloads
        WHERE file_name LIKE ?1
           OR original_url LIKE ?1
           OR COALESCE(final_url, '') LIKE ?1
           OR COALESCE(referrer, '') LIKE ?1
           OR COALESCE(sha256, '') LIKE ?1
           OR COALESCE(source_identity, '') LIKE ?1
        ORDER BY updated_at DESC, id DESC
        "#
    } else {
        r#"
        SELECT id, capture_key, original_url, final_url, referrer, local_path,
               file_name, mime_type, bytes, started_at, completed_at, sha256,
               status, source_identity, version_number, duplicate_of_id, local_state, updated_at
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
            source_identity: row.get(13)?,
            version_number: row.get(14)?,
            duplicate_of_id: row.get(15)?,
            local_state: row.get(16)?,
            updated_at: row.get(17)?,
        })
    };

    if let Some(search) = search {
        statement.query_map([search], mapper)?.collect()
    } else {
        statement.query_map([], mapper)?.collect()
    }
}

pub fn verify_local_files(path: &Path) -> Result<VerificationSummary, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    verify_local_files_with_connection(&connection).map_err(|error| error.to_string())
}

fn verify_local_files_with_connection(
    connection: &Connection,
) -> rusqlite::Result<VerificationSummary> {
    initialize_connection(connection)?;
    let records = {
        let mut statement = connection.prepare("SELECT id, local_path, sha256 FROM downloads")?;
        let records = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        records
    };

    let mut summary = VerificationSummary {
        checked: 0,
        present: 0,
        modified: 0,
        missing: 0,
        unavailable: 0,
    };

    for (id, local_path, baseline_hash) in records {
        summary.checked += 1;
        let path = Path::new(&local_path);
        let local_state = if !path.is_file() {
            summary.missing += 1;
            "LOCAL_MISSING"
        } else if let Some(expected) = baseline_hash.as_deref() {
            match sha256_file(path) {
                Ok(current) if current != expected => {
                    summary.modified += 1;
                    "LOCAL_MODIFIED"
                }
                Ok(_) => {
                    summary.present += 1;
                    "PRESENT"
                }
                Err(_) => {
                    summary.unavailable += 1;
                    "PRESENT"
                }
            }
        } else {
            summary.present += 1;
            "PRESENT"
        };

        connection.execute(
            "UPDATE downloads SET local_state = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![local_state, id],
        )?;
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("originkeep-{unique}-{name}"))
    }

    fn capture(key: &str, url: &str, path: &Path) -> DownloadCapture {
        DownloadCapture {
            capture_key: key.into(),
            browser_download_id: key.bytes().map(i64::from).sum(),
            original_url: url.into(),
            final_url: None,
            referrer: Some("https://example.com/reports".into()),
            local_path: path.display().to_string(),
            file_name: path.file_name().unwrap().to_string_lossy().into_owned(),
            mime_type: Some("application/pdf".into()),
            bytes: fs::metadata(path)
                .ok()
                .map(|metadata| metadata.len() as i64),
            started_at: Some("2026-09-05T00:00:00Z".into()),
            completed_at: Some("2026-09-05T00:00:01Z".into()),
            state: "complete".into(),
        }
    }

    #[test]
    fn hashes_files_deterministically() {
        let path = unique_path("sha.txt");
        fs::write(&path, b"hello world").unwrap();
        let hash = sha256_file(&path).unwrap();
        fs::remove_file(path).ok();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn canonicalization_is_conservative_and_deterministic() {
        assert_eq!(
            canonicalize_source_url(
                "HTTPS://User:Pass@Example.COM:443/report.pdf?token=abc#page=2"
            )
            .as_deref(),
            Some("https://example.com/report.pdf?token=abc")
        );
        assert_ne!(
            canonicalize_source_url("https://example.com/report.pdf?v=1"),
            canonicalize_source_url("https://example.com/report.pdf?v=2")
        );
        assert_eq!(canonicalize_source_url("file:///tmp/report.pdf"), None);
    }

    #[test]
    fn assigns_duplicates_and_versions_without_filename_guessing() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();
        let first_path = unique_path("report-a.pdf");
        let duplicate_path = unique_path("renamed-copy.pdf");
        let changed_path = unique_path("report-b.pdf");
        fs::write(&first_path, b"version one").unwrap();
        fs::write(&duplicate_path, b"version one").unwrap();
        fs::write(&changed_path, b"version two").unwrap();

        let first = ingest_capture_with_connection(
            &connection,
            &capture(
                "capture-1",
                "https://example.com/report.pdf#one",
                &first_path,
            ),
        )
        .unwrap();
        let duplicate = ingest_capture_with_connection(
            &connection,
            &capture(
                "capture-2",
                "https://example.com/report.pdf#two",
                &duplicate_path,
            ),
        )
        .unwrap();
        let changed = ingest_capture_with_connection(
            &connection,
            &capture("capture-3", "https://example.com/report.pdf", &changed_path),
        )
        .unwrap();

        let rows = list_downloads_with_connection(&connection, None).unwrap();
        let first_row = rows.iter().find(|row| row.id == first.id).unwrap();
        fs::remove_file(first_path).ok();
        fs::remove_file(duplicate_path).ok();
        fs::remove_file(changed_path).ok();

        assert_eq!(first.version_number, Some(1));
        assert_eq!(duplicate.duplicate_of_id, Some(first.id));
        assert_eq!(duplicate.version_number, Some(1));
        assert_eq!(duplicate.status, "DUPLICATE");
        assert_eq!(changed.version_number, Some(2));
        assert_eq!(first_row.status, "SUPERSEDED");
    }

    #[test]
    fn local_verification_preserves_the_download_fingerprint() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_connection(&connection).unwrap();
        let path = unique_path("mutable.txt");
        fs::write(&path, b"original").unwrap();
        let item = capture("capture-local", "https://example.com/mutable.txt", &path);
        let ingested = ingest_capture_with_connection(&connection, &item).unwrap();
        let baseline = ingested.sha256.clone().unwrap();

        fs::write(&path, b"modified locally").unwrap();
        let summary = verify_local_files_with_connection(&connection).unwrap();
        let rows = list_downloads_with_connection(&connection, None).unwrap();
        fs::remove_file(path).ok();

        assert_eq!(summary.modified, 1);
        assert_eq!(rows[0].local_state, "LOCAL_MODIFIED");
        assert_eq!(rows[0].sha256.as_deref(), Some(baseline.as_str()));
    }
}
