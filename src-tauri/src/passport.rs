use crate::{model::DownloadCapture, phase4, storage};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use url::Url;

const PASSPORT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS file_locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    download_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 0,
    first_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(download_id, path),
    FOREIGN KEY(download_id) REFERENCES downloads(id)
);
CREATE INDEX IF NOT EXISTS idx_file_locations_download_id ON file_locations(download_id, is_current DESC, id DESC);
"#;
const PASSPORT_FORMAT: &str = "org.originkeep.passport";
const PASSPORT_VERSION: u32 = 1;
const MAX_SCAN_ENTRIES: usize = 20_000;
const MAX_SCAN_DEPTH: usize = 8;
const RETENTION_POLICIES: [&str; 5] = [
    "MANUAL",
    "REVIEW_WHEN_NEWER",
    "ARCHIVE_WHEN_SUPERSEDED",
    "ARCHIVE_WHEN_EXPIRED",
    "NEVER_ARCHIVE",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLocation {
    pub path: String,
    pub is_current: bool,
    pub first_seen: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportRecord {
    pub download_id: i64,
    pub file_name: String,
    pub local_path: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub sha256: Option<String>,
    pub status: String,
    pub source_identity: Option<String>,
    pub version_number: Option<i64>,
    pub duplicate_of_id: Option<i64>,
    pub local_state: String,
    pub original_url: String,
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub page_url: Option<String>,
    pub page_title: Option<String>,
    pub link_text: Option<String>,
    pub context_text: Option<String>,
    pub browser_name: Option<String>,
    pub completed_at: Option<String>,
    pub purpose: Option<String>,
    pub note: Option<String>,
    pub expires_at: Option<String>,
    pub retention_policy: String,
    pub latest_remote_state: Option<String>,
    pub latest_remote_checked_at: Option<String>,
    pub lifecycle_state: String,
    pub locations: Vec<FileLocation>,
    pub portable_passport_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportExport {
    pub download_id: i64,
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkResult {
    pub download_id: i64,
    pub found: bool,
    pub scanned_entries: usize,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginGraph {
    pub nodes: Vec<OriginNode>,
    pub edges: Vec<OriginEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortablePassport {
    format: String,
    version: u32,
    exported_at: String,
    file: PortableFile,
    origin: PortableOrigin,
    lineage: PortableLineage,
    intent: PortableIntent,
    evidence: PortableEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableFile {
    file_name: String,
    mime_type: Option<String>,
    bytes: Option<i64>,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableOrigin {
    original_url: String,
    final_url: Option<String>,
    referrer: Option<String>,
    source_identity: Option<String>,
    page_url: Option<String>,
    page_title: Option<String>,
    link_text: Option<String>,
    context_text: Option<String>,
    browser_name: Option<String>,
    completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableLineage {
    version_number: Option<i64>,
    duplicate_of_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableIntent {
    purpose: Option<String>,
    note: Option<String>,
    expires_at: Option<String>,
    retention_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortableEvidence {
    latest_remote_state: Option<String>,
    latest_remote_checked_at: Option<String>,
}

pub fn initialize_database(path: &Path) -> Result<(), String> {
    phase4::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())
}

fn initialize_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(PASSPORT_SCHEMA)?;
    connection.execute("UPDATE file_locations SET is_current = 0", [])?;
    connection.execute(
        r#"
        INSERT INTO file_locations (download_id, path, is_current)
        SELECT d.id, d.local_path,
               CASE WHEN COALESCE(l.state, 'ACTIVE') = 'ARCHIVED' THEN 0 ELSE 1 END
        FROM downloads d
        LEFT JOIN lifecycle_entries l ON l.download_id = d.id
        WHERE d.local_path <> ''
        ON CONFLICT(download_id, path) DO UPDATE SET
            is_current = excluded.is_current,
            last_seen = CURRENT_TIMESTAMP
        "#,
        [],
    )?;
    Ok(())
}

pub fn list_passports(path: &Path) -> Result<Vec<PassportRecord>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let ids = {
        let mut statement = connection
            .prepare("SELECT id FROM downloads ORDER BY updated_at DESC, id DESC")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    ids.into_iter()
        .map(|id| load_passport(&connection, id).map_err(|error| error.to_string()))
        .collect()
}

pub fn get_passport(path: &Path, download_id: i64) -> Result<PassportRecord, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    load_passport(&connection, download_id).map_err(|error| error.to_string())
}

fn load_passport(connection: &Connection, download_id: i64) -> rusqlite::Result<PassportRecord> {
    let mut record = connection.query_row(
        r#"
        SELECT d.id, d.file_name, d.local_path, d.mime_type, d.bytes, d.sha256,
               d.status, d.source_identity, d.version_number, d.duplicate_of_id,
               d.local_state, d.original_url, d.final_url, d.referrer,
               d.page_url, d.page_title, d.link_text, d.context_text, d.browser_name,
               d.completed_at, d.purpose, d.note, d.expires_at,
               COALESCE(d.retention_policy, 'MANUAL'), COALESCE(l.state, 'ACTIVE')
        FROM downloads d
        LEFT JOIN lifecycle_entries l ON l.download_id = d.id
        WHERE d.id = ?1
        "#,
        [download_id],
        |row| {
            Ok(PassportRecord {
                download_id: row.get(0)?,
                file_name: row.get(1)?,
                local_path: row.get(2)?,
                mime_type: row.get(3)?,
                bytes: row.get(4)?,
                sha256: row.get(5)?,
                status: row.get(6)?,
                source_identity: row.get(7)?,
                version_number: row.get(8)?,
                duplicate_of_id: row.get(9)?,
                local_state: row.get(10)?,
                original_url: row.get(11)?,
                final_url: row.get(12)?,
                referrer: row.get(13)?,
                page_url: row.get(14)?,
                page_title: row.get(15)?,
                link_text: row.get(16)?,
                context_text: row.get(17)?,
                browser_name: row.get(18)?,
                completed_at: row.get(19)?,
                purpose: row.get(20)?,
                note: row.get(21)?,
                expires_at: row.get(22)?,
                retention_policy: row.get(23)?,
                latest_remote_state: None,
                latest_remote_checked_at: None,
                lifecycle_state: row.get(24)?,
                locations: Vec::new(),
                portable_passport_path: None,
            })
        },
    )?;

    let remote: Option<(String, String)> = connection
        .query_row(
            "SELECT result_state, checked_at FROM remote_checks WHERE download_id = ?1 ORDER BY id DESC LIMIT 1",
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((state, checked_at)) = remote {
        record.latest_remote_state = Some(state);
        record.latest_remote_checked_at = Some(checked_at);
    }
    record.locations = load_locations(connection, download_id)?;
    let sidecar = passport_sidecar_path(Path::new(&record.local_path));
    if sidecar.is_file() {
        record.portable_passport_path = Some(sidecar.display().to_string());
    }
    Ok(record)
}

fn load_locations(
    connection: &Connection,
    download_id: i64,
) -> rusqlite::Result<Vec<FileLocation>> {
    let mut statement = connection.prepare(
        "SELECT path, is_current, first_seen, last_seen FROM file_locations WHERE download_id = ?1 ORDER BY is_current DESC, last_seen DESC, id DESC",
    )?;
    let rows = statement
        .query_map([download_id], |row| {
            Ok(FileLocation {
                path: row.get(0)?,
                is_current: row.get::<_, i64>(1)? != 0,
                first_seen: row.get(2)?,
                last_seen: row.get(3)?,
            })
        })?
        .collect();
    rows
}

pub fn update_metadata(
    path: &Path,
    download_id: i64,
    purpose: Option<String>,
    note: Option<String>,
    expires_at: Option<String>,
    retention_policy: String,
) -> Result<PassportRecord, String> {
    initialize_database(path)?;
    let retention_policy = normalize_retention(&retention_policy)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let changed = connection
        .execute(
            r#"
            UPDATE downloads
            SET purpose = ?1, note = ?2, expires_at = ?3, retention_policy = ?4,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?5
            "#,
            params![
                clean_optional(purpose),
                clean_optional(note),
                clean_optional(expires_at),
                retention_policy,
                download_id
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err(format!("Download record #{download_id} does not exist"));
    }
    load_passport(&connection, download_id).map_err(|error| error.to_string())
}

fn normalize_retention(value: &str) -> Result<String, String> {
    let value = value.trim().to_ascii_uppercase();
    if RETENTION_POLICIES.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(format!(
            "Unsupported retention policy: {value}. Expected one of {}",
            RETENTION_POLICIES.join(", ")
        ))
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.chars().take(4000).collect())
        }
    })
}

pub fn export_passport(path: &Path, download_id: i64) -> Result<PassportExport, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let record = load_passport(&connection, download_id).map_err(|error| error.to_string())?;
    let expected = record
        .sha256
        .clone()
        .ok_or_else(|| "A portable passport requires a recorded SHA-256 fingerprint".to_string())?;
    let local_path = PathBuf::from(&record.local_path);
    if !local_path.is_file() {
        return Err(
            "Restore or relink the local file before exporting its portable passport".into(),
        );
    }
    let current = storage::sha256_file(&local_path)
        .map_err(|error| format!("Could not fingerprint {}: {error}", local_path.display()))?;
    if current != expected {
        return Err("Portable passport export is blocked because local bytes no longer match the recorded fingerprint".into());
    }

    let duplicate_of_sha256 = record.duplicate_of_id.and_then(|id| {
        connection
            .query_row("SELECT sha256 FROM downloads WHERE id = ?1", [id], |row| {
                row.get::<_, Option<String>>(0)
            })
            .optional()
            .ok()
            .flatten()
            .flatten()
    });
    let exported_at: String = connection
        .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    let portable = PortablePassport {
        format: PASSPORT_FORMAT.into(),
        version: PASSPORT_VERSION,
        exported_at,
        file: PortableFile {
            file_name: record.file_name.clone(),
            mime_type: record.mime_type.clone(),
            bytes: record.bytes,
            sha256: expected.clone(),
        },
        origin: PortableOrigin {
            original_url: redact_portable_url(&record.original_url),
            final_url: record.final_url.as_deref().map(redact_portable_url),
            referrer: record.referrer.as_deref().map(redact_portable_url),
            source_identity: record.source_identity.as_deref().map(redact_portable_url),
            page_url: record.page_url.as_deref().map(redact_portable_url),
            page_title: record.page_title.clone(),
            link_text: record.link_text.clone(),
            context_text: record.context_text.clone(),
            browser_name: record.browser_name.clone(),
            completed_at: record.completed_at.clone(),
        },
        lineage: PortableLineage {
            version_number: record.version_number,
            duplicate_of_sha256,
        },
        intent: PortableIntent {
            purpose: record.purpose.clone(),
            note: record.note.clone(),
            expires_at: record.expires_at.clone(),
            retention_policy: record.retention_policy.clone(),
        },
        evidence: PortableEvidence {
            latest_remote_state: record.latest_remote_state.clone(),
            latest_remote_checked_at: record.latest_remote_checked_at.clone(),
        },
    };
    let target = passport_sidecar_path(&local_path);
    let payload = serde_json::to_vec_pretty(&portable).map_err(|error| error.to_string())?;
    fs::write(&target, &payload)
        .map_err(|error| format!("Could not write {}: {error}", target.display()))?;
    Ok(PassportExport {
        download_id,
        path: target.display().to_string(),
        sha256: expected,
        bytes: payload.len() as u64,
    })
}

fn redact_portable_url(value: &str) -> String {
    let Ok(mut url) = Url::parse(value) else {
        return value.to_string();
    };
    if !matches!(url.scheme(), "http" | "https") {
        return value.to_string();
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    let pairs = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return url.to_string();
    }
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in pairs {
            let output = if is_sensitive_query_key(&key) {
                "[REDACTED]"
            } else {
                value.as_str()
            };
            query.append_pair(&key, output);
        }
    }
    url.to_string()
}

fn is_sensitive_query_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "token"
            | "access_token"
            | "id_token"
            | "auth"
            | "authorization"
            | "signature"
            | "sig"
            | "key"
            | "api_key"
            | "apikey"
            | "code"
            | "session"
            | "jwt"
            | "x-amz-signature"
            | "x-amz-credential"
            | "x-amz-security-token"
            | "x-goog-signature"
            | "x-goog-credential"
    )
}

pub fn import_passport(
    path: &Path,
    passport_path: String,
    file_path: String,
) -> Result<PassportRecord, String> {
    initialize_database(path)?;
    let passport_path = PathBuf::from(passport_path);
    let file_path = PathBuf::from(file_path);
    if !passport_path.is_file() || !file_path.is_file() {
        return Err(
            "Passport import requires both an existing Passport JSON and local file".into(),
        );
    }
    let portable: PortablePassport = serde_json::from_slice(
        &fs::read(&passport_path)
            .map_err(|error| format!("Could not read {}: {error}", passport_path.display()))?,
    )
    .map_err(|error| format!("Invalid OriginKeep passport JSON: {error}"))?;
    if portable.format != PASSPORT_FORMAT || portable.version != PASSPORT_VERSION {
        return Err(format!(
            "Unsupported passport format/version: {} v{}",
            portable.format, portable.version
        ));
    }
    let retention_policy = normalize_retention(&portable.intent.retention_policy)?;
    let actual = storage::sha256_file(&file_path)
        .map_err(|error| format!("Could not fingerprint {}: {error}", file_path.display()))?;
    if actual != portable.file.sha256 {
        return Err(
            "The selected file does not match the SHA-256 recorded by this passport".into(),
        );
    }
    let metadata = fs::metadata(&file_path).map_err(|error| error.to_string())?;
    let capture = DownloadCapture {
        capture_key: format!(
            "passport-import:{}:{}",
            portable.file.sha256,
            file_path.display()
        ),
        browser_download_id: 0,
        original_url: portable.origin.original_url.clone(),
        final_url: portable.origin.final_url.clone(),
        referrer: portable.origin.referrer.clone(),
        local_path: file_path.display().to_string(),
        file_name: file_path
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| portable.file.file_name.clone()),
        mime_type: portable.file.mime_type.clone(),
        bytes: Some(metadata.len() as i64),
        started_at: portable.origin.completed_at.clone(),
        completed_at: portable.origin.completed_at.clone(),
        state: "complete".into(),
        page_url: portable.origin.page_url.clone(),
        page_title: portable.origin.page_title.clone(),
        link_text: portable.origin.link_text.clone(),
        context_text: portable.origin.context_text.clone(),
        browser_name: portable.origin.browser_name.clone(),
    };
    let ingested = storage::ingest_capture(path, &capture)?;
    update_metadata(
        path,
        ingested.id,
        portable.intent.purpose,
        portable.intent.note,
        portable.intent.expires_at,
        retention_policy,
    )?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())?;
    record_location(&connection, ingested.id, &file_path).map_err(|error| error.to_string())?;
    load_passport(&connection, ingested.id).map_err(|error| error.to_string())
}

pub fn relink_download(
    path: &Path,
    download_id: i64,
    candidate_path: String,
) -> Result<PassportRecord, String> {
    initialize_database(path)?;
    let candidate = PathBuf::from(candidate_path);
    if !candidate.is_file() {
        return Err(format!(
            "Candidate path is not a file: {}",
            candidate.display()
        ));
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let (expected, old_path, lifecycle_state): (Option<String>, String, String) = connection
        .query_row(
            r#"
            SELECT d.sha256, d.local_path, COALESCE(l.state, 'ACTIVE')
            FROM downloads d LEFT JOIN lifecycle_entries l ON l.download_id = d.id
            WHERE d.id = ?1
            "#,
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|error| error.to_string())?;
    if lifecycle_state == "ARCHIVED" {
        return Err("Restore the archived copy before relinking this record".into());
    }
    let expected =
        expected.ok_or_else(|| "Relinking requires a recorded SHA-256 fingerprint".to_string())?;
    let current = storage::sha256_file(&candidate)
        .map_err(|error| format!("Could not fingerprint {}: {error}", candidate.display()))?;
    if current != expected {
        return Err("Candidate file does not match the recorded SHA-256; OriginKeep will not relink by filename alone".into());
    }
    let metadata = fs::metadata(&candidate).map_err(|error| error.to_string())?;
    let name = candidate
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| "Candidate path has no filename".to_string())?;
    connection
        .execute(
            r#"
            UPDATE downloads
            SET local_path = ?1, file_name = ?2, bytes = ?3, local_state = 'PRESENT',
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?4
            "#,
            params![
                candidate.display().to_string(),
                name,
                metadata.len() as i64,
                download_id
            ],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE file_locations SET is_current = 0, last_seen = CURRENT_TIMESTAMP WHERE download_id = ?1",
            [download_id],
        )
        .map_err(|error| error.to_string())?;
    record_location_with_state(&connection, download_id, Path::new(&old_path), false)
        .map_err(|error| error.to_string())?;
    record_location(&connection, download_id, &candidate).map_err(|error| error.to_string())?;
    load_passport(&connection, download_id).map_err(|error| error.to_string())
}

pub fn find_moved_file(
    path: &Path,
    download_id: i64,
    search_root: String,
) -> Result<RelinkResult, String> {
    initialize_database(path)?;
    let root = PathBuf::from(search_root);
    if !root.is_dir() {
        return Err(format!(
            "Search root is not a directory: {}",
            root.display()
        ));
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let (expected, expected_bytes): (Option<String>, Option<i64>) = connection
        .query_row(
            "SELECT sha256, bytes FROM downloads WHERE id = ?1",
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    let expected = expected.ok_or_else(|| {
        "Moved-file discovery requires a recorded SHA-256 fingerprint".to_string()
    })?;
    let mut scanned = 0usize;
    let found = scan_directory_for_hash(
        &root,
        &expected,
        expected_bytes.and_then(|value| u64::try_from(value).ok()),
        0,
        &mut scanned,
    )?;
    if let Some(found) = found {
        let display = found.display().to_string();
        relink_download(path, download_id, display.clone())?;
        Ok(RelinkResult {
            download_id,
            found: true,
            scanned_entries: scanned,
            path: Some(display),
            message: "OriginKeep found the same SHA-256 at a new path and relinked it without filename guessing.".into(),
        })
    } else {
        Ok(RelinkResult {
            download_id,
            found: false,
            scanned_entries: scanned,
            path: None,
            message: format!(
                "No exact SHA-256 match was found within the bounded scan (max {MAX_SCAN_ENTRIES} entries, depth {MAX_SCAN_DEPTH})."
            ),
        })
    }
}

fn scan_directory_for_hash(
    root: &Path,
    expected_hash: &str,
    expected_bytes: Option<u64>,
    depth: usize,
    scanned: &mut usize,
) -> Result<Option<PathBuf>, String> {
    if depth > MAX_SCAN_DEPTH || *scanned >= MAX_SCAN_ENTRIES {
        return Ok(None);
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(None),
    };
    for entry in entries {
        if *scanned >= MAX_SCAN_ENTRIES {
            break;
        }
        let Ok(entry) = entry else { continue };
        *scanned += 1;
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if let Some(found) =
                scan_directory_for_hash(&path, expected_hash, expected_bytes, depth + 1, scanned)?
            {
                return Ok(Some(found));
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if let Some(expected_bytes) = expected_bytes {
            if fs::metadata(&path).map(|value| value.len()).ok() != Some(expected_bytes) {
                continue;
            }
        }
        if storage::sha256_file(&path).ok().as_deref() == Some(expected_hash) {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn record_location(connection: &Connection, download_id: i64, path: &Path) -> rusqlite::Result<()> {
    record_location_with_state(connection, download_id, path, true)
}

fn record_location_with_state(
    connection: &Connection,
    download_id: i64,
    path: &Path,
    current: bool,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO file_locations (download_id, path, is_current)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(download_id, path) DO UPDATE SET
            is_current = excluded.is_current,
            last_seen = CURRENT_TIMESTAMP
        "#,
        params![download_id, path.display().to_string(), i64::from(current)],
    )?;
    Ok(())
}

fn passport_sidecar_path(file: &Path) -> PathBuf {
    PathBuf::from(format!("{}.originkeep.json", file.display()))
}

pub fn origin_graph(path: &Path) -> Result<OriginGraph, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    type GraphRow = (
        i64,
        String,
        String,
        Option<String>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    );
    let records: Vec<GraphRow> = {
        let mut statement = connection
            .prepare(
                "SELECT id, file_name, original_url, source_identity, version_number, duplicate_of_id, sha256, purpose FROM downloads ORDER BY id ASC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        rows
    };

    let mut nodes = BTreeMap::<String, OriginNode>::new();
    let mut edges = BTreeSet::<(String, String, String)>::new();
    let mut source_ids = BTreeMap::<String, String>::new();
    let mut last_version_by_source = BTreeMap::<String, (i64, String)>::new();

    for (
        id,
        file_name,
        original_url,
        source_identity,
        version_number,
        duplicate_of_id,
        sha256,
        purpose,
    ) in records
    {
        let source_value = source_identity
            .clone()
            .unwrap_or_else(|| original_url.clone());
        let host = if original_url == "urn:originkeep:local-adoption" {
            "Local adoption".to_string()
        } else {
            Url::parse(&source_value)
                .ok()
                .and_then(|url| url.host_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "Unknown source".into())
        };
        let site_id = format!("site:{host}");
        nodes.entry(site_id.clone()).or_insert_with(|| OriginNode {
            id: site_id.clone(),
            kind: "SITE".into(),
            label: host,
            detail: None,
        });

        let next_source_id = format!("source:{}", source_ids.len() + 1);
        let source_id = source_ids
            .entry(source_value.clone())
            .or_insert(next_source_id)
            .clone();
        nodes
            .entry(source_id.clone())
            .or_insert_with(|| OriginNode {
                id: source_id.clone(),
                kind: "SOURCE".into(),
                label: source_value.clone(),
                detail: None,
            });
        edges.insert((site_id, source_id.clone(), "ORIGIN".into()));

        let file_id = format!("file:{id}");
        let detail = format!(
            "{}{}{}",
            version_number
                .map(|value| format!("v{value}"))
                .unwrap_or_else(|| "unversioned".into()),
            purpose
                .as_deref()
                .map(|value| format!(" · {value}"))
                .unwrap_or_default(),
            sha256
                .as_deref()
                .map(|value| format!(" · {}…", &value[..value.len().min(12)]))
                .unwrap_or_default()
        );
        nodes.insert(
            file_id.clone(),
            OriginNode {
                id: file_id.clone(),
                kind: "FILE".into(),
                label: file_name,
                detail: Some(detail),
            },
        );
        edges.insert((source_id, file_id.clone(), "HAS_VERSION".into()));
        if let Some(duplicate_id) = duplicate_of_id {
            edges.insert((
                file_id.clone(),
                format!("file:{duplicate_id}"),
                "EXACT_DUPLICATE_OF".into(),
            ));
        }
        if let Some(version) = version_number {
            if let Some((previous_version, previous_file)) =
                last_version_by_source.get(&source_value)
            {
                if version > *previous_version {
                    edges.insert((
                        previous_file.clone(),
                        file_id.clone(),
                        "NEXT_VERSION".into(),
                    ));
                }
            }
            if duplicate_of_id.is_none() {
                let replace = last_version_by_source
                    .get(&source_value)
                    .is_none_or(|(existing, _)| version >= *existing);
                if replace {
                    last_version_by_source.insert(source_value, (version, file_id));
                }
            }
        }
    }

    Ok(OriginGraph {
        nodes: nodes.into_values().collect(),
        edges: edges
            .into_iter()
            .map(|(from, to, kind)| OriginEdge { from, to, kind })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("originkeep-passport-{unique}-{name}"))
    }

    #[test]
    fn portable_passport_path_is_adjacent_and_explicit() {
        assert_eq!(
            passport_sidecar_path(Path::new("/tmp/report.pdf"))
                .display()
                .to_string(),
            "/tmp/report.pdf.originkeep.json"
        );
    }

    #[test]
    fn portable_urls_redact_common_credentials_without_dropping_identity_queries() {
        let value = redact_portable_url(
            "https://example.com/report?v=2&token=secret&X-Amz-Signature=abc#page=2",
        );
        assert!(value.contains("v=2"));
        assert!(value.contains("token=%5BREDACTED%5D"));
        assert!(value.contains("X-Amz-Signature=%5BREDACTED%5D"));
        assert!(!value.contains("secret"));
    }

    #[test]
    fn bounded_scan_finds_content_after_rename() {
        let root = unique_path("scan");
        fs::create_dir_all(root.join("nested")).unwrap();
        let file = root.join("nested").join("renamed.bin");
        fs::write(&file, b"same content identity").unwrap();
        let hash = storage::sha256_file(&file).unwrap();
        let mut scanned = 0;
        let found = scan_directory_for_hash(&root, &hash, None, 0, &mut scanned).unwrap();
        fs::remove_dir_all(&root).ok();
        assert_eq!(found.as_deref(), Some(file.as_path()));
        assert!(scanned > 0);
    }

    #[test]
    fn metadata_cleanup_and_policy_validation_fail_closed() {
        assert_eq!(clean_optional(Some("   ".into())), None);
        assert_eq!(
            clean_optional(Some(" reference ".into())).as_deref(),
            Some("reference")
        );
        assert!(normalize_retention("never_archive").is_ok());
        assert!(normalize_retention("delete_everything").is_err());
    }
}
