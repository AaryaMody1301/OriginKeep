use crate::{model::DownloadCapture, model::IngestResult, phase4, storage};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

const PASSPORT_SPEC: &str = "org.originkeep.passport.v1";
const MAX_SIDECAR_BYTES: u64 = 1024 * 1024;
const PASSPORT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS passport_context (
    download_id INTEGER PRIMARY KEY,
    browser_name TEXT,
    page_title TEXT,
    page_url TEXT,
    link_text TEXT,
    context_text TEXT,
    context_source TEXT,
    purpose TEXT NOT NULL DEFAULT 'UNSPECIFIED',
    note TEXT,
    expires_at TEXT,
    sigstore_identity TEXT,
    sigstore_issuer TEXT,
    imported_from TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(download_id) REFERENCES downloads(id)
);
CREATE TABLE IF NOT EXISTS file_locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256 TEXT NOT NULL,
    path TEXT NOT NULL,
    first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    state TEXT NOT NULL DEFAULT 'PRESENT',
    UNIQUE(sha256, path)
);
CREATE INDEX IF NOT EXISTS idx_file_locations_sha256 ON file_locations(sha256);
CREATE TABLE IF NOT EXISTS trust_observations (
    download_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    state TEXT NOT NULL,
    summary TEXT NOT NULL,
    details TEXT,
    checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(download_id, kind),
    FOREIGN KEY(download_id) REFERENCES downloads(id)
);
"#;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureContext {
    pub browser_name: Option<String>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub link_text: Option<String>,
    pub context_text: Option<String>,
    pub context_source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportSummary {
    pub download_id: i64,
    pub purpose: String,
    pub expires_at: Option<String>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub location_count: i64,
    pub trust_signal_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLocation {
    pub path: String,
    pub state: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustObservation {
    pub kind: String,
    pub state: String,
    pub summary: String,
    pub details: Option<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePassport {
    pub download_id: i64,
    pub file_name: String,
    pub local_path: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub sha256: Option<String>,
    pub original_url: String,
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub source_identity: Option<String>,
    pub downloaded_at: Option<String>,
    pub version_number: Option<i64>,
    pub duplicate_of_id: Option<i64>,
    pub status: String,
    pub local_state: String,
    pub lifecycle_state: String,
    pub archive_path: Option<String>,
    pub browser_name: Option<String>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub link_text: Option<String>,
    pub context_text: Option<String>,
    pub context_source: Option<String>,
    pub purpose: String,
    pub note: Option<String>,
    pub expires_at: Option<String>,
    pub sigstore_identity: Option<String>,
    pub sigstore_issuer: Option<String>,
    pub remote_state: Option<String>,
    pub remote_evidence: Option<String>,
    pub remote_checked_at: Option<String>,
    pub locations: Vec<FileLocation>,
    pub trust: Vec<TrustObservation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportExportResult {
    pub download_id: i64,
    pub sidecar_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconnectResult {
    pub download_id: i64,
    pub path: String,
    pub primary_path_updated: bool,
    pub location_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationRefreshSummary {
    pub checked: i64,
    pub present: i64,
    pub missing: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortablePassport {
    spec: String,
    exported_at: String,
    sha256: String,
    file_name: String,
    mime_type: Option<String>,
    bytes: Option<i64>,
    original_url: String,
    final_url: Option<String>,
    referrer: Option<String>,
    source_identity: Option<String>,
    downloaded_at: Option<String>,
    version_number: Option<i64>,
    browser_name: Option<String>,
    page_title: Option<String>,
    page_url: Option<String>,
    link_text: Option<String>,
    context_text: Option<String>,
    context_source: Option<String>,
    purpose: String,
    note: Option<String>,
    expires_at: Option<String>,
    trust: Vec<TrustObservation>,
}

#[derive(Debug)]
struct PassportRow {
    download_id: i64,
    file_name: String,
    local_path: String,
    mime_type: Option<String>,
    bytes: Option<i64>,
    sha256: Option<String>,
    original_url: String,
    final_url: Option<String>,
    referrer: Option<String>,
    source_identity: Option<String>,
    downloaded_at: Option<String>,
    version_number: Option<i64>,
    duplicate_of_id: Option<i64>,
    status: String,
    local_state: String,
    lifecycle_state: String,
    archive_path: Option<String>,
    browser_name: Option<String>,
    page_title: Option<String>,
    page_url: Option<String>,
    link_text: Option<String>,
    context_text: Option<String>,
    context_source: Option<String>,
    purpose: String,
    note: Option<String>,
    expires_at: Option<String>,
    sigstore_identity: Option<String>,
    sigstore_issuer: Option<String>,
    remote_state: Option<String>,
    remote_evidence: Option<String>,
    remote_checked_at: Option<String>,
}

pub fn initialize_database(path: &Path) -> Result<(), String> {
    phase4::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())
}

fn initialize_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(PASSPORT_SCHEMA)?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 5 {
        backfill_locations(connection)?;
        connection.execute_batch("PRAGMA user_version = 5;")?;
    }
    Ok(())
}

fn backfill_locations(connection: &Connection) -> rusqlite::Result<()> {
    let rows = {
        let mut statement = connection.prepare(
            "SELECT sha256, local_path, local_state FROM downloads WHERE sha256 IS NOT NULL",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    for (hash, path, state) in rows {
        upsert_location(connection, &hash, &path, location_state(&state))?;
    }
    Ok(())
}

fn location_state(local_state: &str) -> &'static str {
    if local_state == "LOCAL_MISSING" {
        "MISSING"
    } else {
        "PRESENT"
    }
}

fn clean_optional(value: Option<&str>, max_chars: usize) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_chars).collect())
}

fn upsert_location(
    connection: &Connection,
    hash: &str,
    path: &str,
    state: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO file_locations (sha256, path, state)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(sha256, path) DO UPDATE SET
            state = excluded.state,
            last_seen_at = CURRENT_TIMESTAMP
        "#,
        params![hash, path, state],
    )?;
    Ok(())
}

pub fn record_capture(
    path: &Path,
    capture: &DownloadCapture,
    result: &IngestResult,
    context: &CaptureContext,
) -> Result<(), String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute(
            r#"
            INSERT INTO passport_context (
                download_id, browser_name, page_title, page_url, link_text,
                context_text, context_source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(download_id) DO UPDATE SET
                browser_name = COALESCE(excluded.browser_name, passport_context.browser_name),
                page_title = COALESCE(excluded.page_title, passport_context.page_title),
                page_url = COALESCE(excluded.page_url, passport_context.page_url),
                link_text = COALESCE(excluded.link_text, passport_context.link_text),
                context_text = COALESCE(excluded.context_text, passport_context.context_text),
                context_source = COALESCE(excluded.context_source, passport_context.context_source),
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![
                result.id,
                clean_optional(context.browser_name.as_deref(), 80),
                clean_optional(context.page_title.as_deref(), 500),
                clean_optional(context.page_url.as_deref(), 4096),
                clean_optional(context.link_text.as_deref(), 500),
                clean_optional(context.context_text.as_deref(), 2000),
                clean_optional(context.context_source.as_deref(), 80),
            ],
        )
        .map_err(|error| error.to_string())?;
    if let Some(hash) = result.sha256.as_deref() {
        upsert_location(&connection, hash, &capture.local_path, "PRESENT")
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn list_passport_summaries(path: &Path) -> Result<Vec<PassportSummary>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT d.id, COALESCE(pc.purpose, 'UNSPECIFIED'), pc.expires_at,
                   pc.page_title, pc.page_url,
                   CASE WHEN d.sha256 IS NULL THEN 0 ELSE (
                       SELECT COUNT(*) FROM file_locations fl WHERE fl.sha256 = d.sha256
                   ) END,
                   (SELECT COUNT(*) FROM trust_observations t WHERE t.download_id = d.id)
            FROM downloads d
            LEFT JOIN passport_context pc ON pc.download_id = d.id
            ORDER BY d.updated_at DESC, d.id DESC
            "#,
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(PassportSummary {
                download_id: row.get(0)?,
                purpose: row.get(1)?,
                expires_at: row.get(2)?,
                page_title: row.get(3)?,
                page_url: row.get(4)?,
                location_count: row.get(5)?,
                trust_signal_count: row.get(6)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

pub fn get_file_passport(path: &Path, download_id: i64) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let row = load_passport_row(&connection, download_id)?;
    assemble_passport(&connection, row)
}

fn load_passport_row(connection: &Connection, download_id: i64) -> Result<PassportRow, String> {
    connection
        .query_row(
            r#"
            SELECT d.id, d.file_name, d.local_path, d.mime_type, d.bytes, d.sha256,
                   d.original_url, d.final_url, d.referrer, d.source_identity,
                   COALESCE(d.completed_at, d.started_at), d.version_number,
                   d.duplicate_of_id, d.status, d.local_state,
                   COALESCE(le.state, 'ACTIVE'), le.archive_path,
                   pc.browser_name, pc.page_title, pc.page_url, pc.link_text,
                   pc.context_text, pc.context_source,
                   COALESCE(pc.purpose, 'UNSPECIFIED'), pc.note, pc.expires_at,
                   pc.sigstore_identity, pc.sigstore_issuer,
                   rc.result_state, rc.evidence, rc.checked_at
            FROM downloads d
            LEFT JOIN lifecycle_entries le ON le.download_id = d.id
            LEFT JOIN passport_context pc ON pc.download_id = d.id
            LEFT JOIN remote_checks rc ON rc.id = (
                SELECT MAX(id) FROM remote_checks WHERE download_id = d.id
            )
            WHERE d.id = ?1
            "#,
            [download_id],
            |row| {
                Ok(PassportRow {
                    download_id: row.get(0)?,
                    file_name: row.get(1)?,
                    local_path: row.get(2)?,
                    mime_type: row.get(3)?,
                    bytes: row.get(4)?,
                    sha256: row.get(5)?,
                    original_url: row.get(6)?,
                    final_url: row.get(7)?,
                    referrer: row.get(8)?,
                    source_identity: row.get(9)?,
                    downloaded_at: row.get(10)?,
                    version_number: row.get(11)?,
                    duplicate_of_id: row.get(12)?,
                    status: row.get(13)?,
                    local_state: row.get(14)?,
                    lifecycle_state: row.get(15)?,
                    archive_path: row.get(16)?,
                    browser_name: row.get(17)?,
                    page_title: row.get(18)?,
                    page_url: row.get(19)?,
                    link_text: row.get(20)?,
                    context_text: row.get(21)?,
                    context_source: row.get(22)?,
                    purpose: row.get(23)?,
                    note: row.get(24)?,
                    expires_at: row.get(25)?,
                    sigstore_identity: row.get(26)?,
                    sigstore_issuer: row.get(27)?,
                    remote_state: row.get(28)?,
                    remote_evidence: row.get(29)?,
                    remote_checked_at: row.get(30)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Download record #{download_id} does not exist"))
}

fn assemble_passport(connection: &Connection, row: PassportRow) -> Result<FilePassport, String> {
    let locations = if let Some(hash) = row.sha256.as_deref() {
        list_locations(connection, hash)?
    } else {
        Vec::new()
    };
    let trust = list_trust(connection, row.download_id)?;
    Ok(FilePassport {
        download_id: row.download_id,
        file_name: row.file_name,
        local_path: row.local_path,
        mime_type: row.mime_type,
        bytes: row.bytes,
        sha256: row.sha256,
        original_url: row.original_url,
        final_url: row.final_url,
        referrer: row.referrer,
        source_identity: row.source_identity,
        downloaded_at: row.downloaded_at,
        version_number: row.version_number,
        duplicate_of_id: row.duplicate_of_id,
        status: row.status,
        local_state: row.local_state,
        lifecycle_state: row.lifecycle_state,
        archive_path: row.archive_path,
        browser_name: row.browser_name,
        page_title: row.page_title,
        page_url: row.page_url,
        link_text: row.link_text,
        context_text: row.context_text,
        context_source: row.context_source,
        purpose: row.purpose,
        note: row.note,
        expires_at: row.expires_at,
        sigstore_identity: row.sigstore_identity,
        sigstore_issuer: row.sigstore_issuer,
        remote_state: row.remote_state,
        remote_evidence: row.remote_evidence,
        remote_checked_at: row.remote_checked_at,
        locations,
        trust,
    })
}

fn list_locations(connection: &Connection, hash: &str) -> Result<Vec<FileLocation>, String> {
    let mut statement = connection
        .prepare(
            "SELECT path, state, first_seen_at, last_seen_at FROM file_locations WHERE sha256 = ?1 ORDER BY state DESC, last_seen_at DESC, path ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([hash], |row| {
            Ok(FileLocation {
                path: row.get(0)?,
                state: row.get(1)?,
                first_seen_at: row.get(2)?,
                last_seen_at: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

fn list_trust(connection: &Connection, download_id: i64) -> Result<Vec<TrustObservation>, String> {
    let mut statement = connection
        .prepare(
            "SELECT kind, state, summary, details, checked_at FROM trust_observations WHERE download_id = ?1 ORDER BY kind ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([download_id], |row| {
            Ok(TrustObservation {
                kind: row.get(0)?,
                state: row.get(1)?,
                summary: row.get(2)?,
                details: row.get(3)?,
                checked_at: row.get(4)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

fn normalize_purpose(value: &str) -> Result<String, String> {
    let normalized = value
        .trim()
        .to_ascii_uppercase()
        .replace(' ', "_")
        .replace('-', "_");
    match normalized.as_str() {
        "UNSPECIFIED" | "REFERENCE" | "READ_LATER" | "TEMPORARY" | "WORK" | "RECEIPT"
        | "INSTALLER" | "DATASET" | "OTHER" => Ok(normalized),
        _ => Err("Unsupported purpose. Use Reference, Read later, Temporary, Work, Receipt, Installer, Dataset, Other or Unspecified.".into()),
    }
}

pub fn update_passport_metadata(
    path: &Path,
    download_id: i64,
    purpose: String,
    note: Option<String>,
    expires_at: Option<String>,
    sigstore_identity: Option<String>,
    sigstore_issuer: Option<String>,
) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM downloads WHERE id = ?1)",
            [download_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("Download record #{download_id} does not exist"));
    }
    connection
        .execute(
            r#"
            INSERT INTO passport_context (
                download_id, purpose, note, expires_at, sigstore_identity, sigstore_issuer
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(download_id) DO UPDATE SET
                purpose = excluded.purpose,
                note = excluded.note,
                expires_at = excluded.expires_at,
                sigstore_identity = excluded.sigstore_identity,
                sigstore_issuer = excluded.sigstore_issuer,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![
                download_id,
                normalize_purpose(&purpose)?,
                clean_optional(note.as_deref(), 4000),
                clean_optional(expires_at.as_deref(), 80),
                clean_optional(sigstore_identity.as_deref(), 512),
                clean_optional(sigstore_issuer.as_deref(), 1024),
            ],
        )
        .map_err(|error| error.to_string())?;
    get_file_passport(path, download_id)
}

pub fn reconnect_file(
    path: &Path,
    download_id: i64,
    new_path: String,
) -> Result<ReconnectResult, String> {
    initialize_database(path)?;
    let candidate = PathBuf::from(new_path.trim());
    if !candidate.is_file() {
        return Err(format!("Selected path is not a readable file: {}", candidate.display()));
    }
    let actual_hash = storage::sha256_file(&candidate).map_err(|error| error.to_string())?;
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    let (expected_hash, current_path): (Option<String>, String) = connection
        .query_row(
            "SELECT sha256, local_path FROM downloads WHERE id = ?1",
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Download record #{download_id} does not exist"))?;
    let expected_hash = expected_hash.ok_or_else(|| {
        "This record has no immutable SHA-256 baseline, so OriginKeep will not reconnect it by guesswork".to_string()
    })?;
    if actual_hash != expected_hash {
        return Err("The selected file does not match the recorded SHA-256 content identity".into());
    }
    let candidate_text = candidate.display().to_string();
    upsert_location(&connection, &expected_hash, &candidate_text, "PRESENT")
        .map_err(|error| error.to_string())?;
    let primary_path_updated = !Path::new(&current_path).is_file();
    if primary_path_updated {
        let file_name = candidate
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "Selected path has no valid filename".to_string())?
            .to_string();
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE downloads SET local_path = ?1, file_name = ?2, local_state = 'PRESENT', updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                params![candidate_text, file_name, download_id],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE lifecycle_entries SET original_path = ?1, updated_at = CURRENT_TIMESTAMP WHERE download_id = ?2 AND state = 'ACTIVE'",
                params![candidate.display().to_string(), download_id],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }
    let location_count = connection
        .query_row(
            "SELECT COUNT(*) FROM file_locations WHERE sha256 = ?1",
            [&expected_hash],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(ReconnectResult {
        download_id,
        path: candidate.display().to_string(),
        primary_path_updated,
        location_count,
    })
}

pub fn refresh_locations(path: &Path) -> Result<LocationRefreshSummary, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let rows = {
        let mut statement = connection
            .prepare("SELECT id, path FROM file_locations ORDER BY id ASC")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    let mut summary = LocationRefreshSummary {
        checked: 0,
        present: 0,
        missing: 0,
    };
    for (id, location) in rows {
        summary.checked += 1;
        let state = if Path::new(&location).is_file() {
            summary.present += 1;
            "PRESENT"
        } else {
            summary.missing += 1;
            "MISSING"
        };
        connection
            .execute(
                "UPDATE file_locations SET state = ?1, last_seen_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![state, id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(summary)
}

fn current_timestamp(path: &Path) -> Result<String, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .query_row("SELECT CURRENT_TIMESTAMP", [], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn usable_file_path(passport: &FilePassport) -> Option<PathBuf> {
    let primary = PathBuf::from(&passport.local_path);
    if primary.is_file() {
        return Some(primary);
    }
    passport
        .archive_path
        .as_deref()
        .map(PathBuf::from)
        .filter(|path| path.is_file())
}

pub fn export_passport(path: &Path, download_id: i64) -> Result<PassportExportResult, String> {
    refresh_trust(path, download_id)?;
    let passport = get_file_passport(path, download_id)?;
    let hash = passport
        .sha256
        .clone()
        .ok_or_else(|| "A portable passport requires an immutable SHA-256 fingerprint".to_string())?;
    let source_path = usable_file_path(&passport)
        .ok_or_else(|| "No local or archived copy is available for passport export".to_string())?;
    let portable = PortablePassport {
        spec: PASSPORT_SPEC.into(),
        exported_at: current_timestamp(path)?,
        sha256: hash.clone(),
        file_name: passport.file_name.clone(),
        mime_type: passport.mime_type.clone(),
        bytes: passport.bytes,
        original_url: passport.original_url.clone(),
        final_url: passport.final_url.clone(),
        referrer: passport.referrer.clone(),
        source_identity: passport.source_identity.clone(),
        downloaded_at: passport.downloaded_at.clone(),
        version_number: passport.version_number,
        browser_name: passport.browser_name.clone(),
        page_title: passport.page_title.clone(),
        page_url: passport.page_url.clone(),
        link_text: passport.link_text.clone(),
        context_text: passport.context_text.clone(),
        context_source: passport.context_source.clone(),
        purpose: passport.purpose.clone(),
        note: passport.note.clone(),
        expires_at: passport.expires_at.clone(),
        trust: passport.trust.clone(),
    };
    let sidecar = PathBuf::from(format!("{}.originkeep.json", source_path.display()));
    let json = serde_json::to_string_pretty(&portable).map_err(|error| error.to_string())?;
    fs::write(&sidecar, json).map_err(|error| error.to_string())?;
    Ok(PassportExportResult {
        download_id,
        sidecar_path: sidecar.display().to_string(),
        sha256: hash,
    })
}

pub fn import_passport(path: &Path, sidecar_path: String) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let sidecar = PathBuf::from(sidecar_path.trim());
    let metadata = fs::metadata(&sidecar).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_SIDECAR_BYTES {
        return Err("OriginKeep passport sidecars are limited to 1 MiB".into());
    }
    let raw = fs::read_to_string(&sidecar).map_err(|error| error.to_string())?;
    let portable: PortablePassport = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    if portable.spec != PASSPORT_SPEC {
        return Err(format!("Unsupported passport spec: {}", portable.spec));
    }
    let sidecar_text = sidecar.display().to_string();
    let file_text = sidecar_text
        .strip_suffix(".originkeep.json")
        .ok_or_else(|| "Passport filename must end in .originkeep.json".to_string())?;
    let file_path = PathBuf::from(file_text);
    if !file_path.is_file() {
        return Err(format!("Passport file is not beside the sidecar: {}", file_path.display()));
    }
    let actual_hash = storage::sha256_file(&file_path).map_err(|error| error.to_string())?;
    if actual_hash != portable.sha256 {
        return Err("Portable passport SHA-256 does not match the adjacent file".into());
    }
    let bytes = fs::metadata(&file_path)
        .ok()
        .and_then(|metadata| i64::try_from(metadata.len()).ok());
    let capture = DownloadCapture {
        capture_key: format!("passport:{}:{}", portable.sha256, file_path.display()),
        browser_download_id: 0,
        original_url: portable.original_url.clone(),
        final_url: portable.final_url.clone(),
        referrer: portable.referrer.clone(),
        local_path: file_path.display().to_string(),
        file_name: file_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&portable.file_name)
            .to_string(),
        mime_type: portable.mime_type.clone(),
        bytes,
        started_at: portable.downloaded_at.clone(),
        completed_at: portable.downloaded_at.clone(),
        state: "complete".into(),
    };
    let result = storage::ingest_capture(path, &capture)?;
    let context = CaptureContext {
        browser_name: portable.browser_name.clone(),
        page_title: portable.page_title.clone(),
        page_url: portable.page_url.clone(),
        link_text: portable.link_text.clone(),
        context_text: portable.context_text.clone(),
        context_source: Some("portable-passport".into()),
    };
    record_capture(path, &capture, &result, &context)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute(
            "UPDATE passport_context SET purpose = ?1, note = ?2, expires_at = ?3, imported_from = ?4, updated_at = CURRENT_TIMESTAMP WHERE download_id = ?5",
            params![
                normalize_purpose(&portable.purpose)?,
                clean_optional(portable.note.as_deref(), 4000),
                clean_optional(portable.expires_at.as_deref(), 80),
                sidecar.display().to_string(),
                result.id,
            ],
        )
        .map_err(|error| error.to_string())?;
    get_file_passport(path, result.id)
}

fn observation(kind: &str, state: &str, summary: String, details: Option<String>) -> TrustObservation {
    TrustObservation {
        kind: kind.into(),
        state: state.into(),
        summary,
        details,
        checked_at: "pending persistence".into(),
    }
}

fn integrity_observation(path: &Path, expected: Option<&str>) -> TrustObservation {
    let Some(expected) = expected else {
        return observation(
            "LOCAL_INTEGRITY",
            "NO_BASELINE",
            "No download-time SHA-256 baseline is available.".into(),
            None,
        );
    };
    match storage::sha256_file(path) {
        Ok(actual) if actual == expected => observation(
            "LOCAL_INTEGRITY",
            "VERIFIED",
            "Current bytes match the immutable download-time SHA-256.".into(),
            Some(actual),
        ),
        Ok(actual) => observation(
            "LOCAL_INTEGRITY",
            "MODIFIED",
            "Current bytes do not match the download-time SHA-256.".into(),
            Some(format!("expected {expected}; actual {actual}")),
        ),
        Err(error) => observation(
            "LOCAL_INTEGRITY",
            "CHECK_FAILED",
            "OriginKeep could not hash the current file.".into(),
            Some(error.to_string()),
        ),
    }
}

#[cfg(windows)]
fn mark_of_web_observation(path: &Path) -> TrustObservation {
    let ads = format!("{}:Zone.Identifier", path.display());
    match fs::read_to_string(&ads) {
        Ok(content) => {
            let value = |key: &str| {
                content.lines().find_map(|line| {
                    let (candidate, value) = line.split_once('=')?;
                    candidate
                        .trim()
                        .eq_ignore_ascii_case(key)
                        .then(|| value.trim().to_string())
                })
            };
            let zone = value("ZoneId").unwrap_or_else(|| "unknown".into());
            observation(
                "WINDOWS_ORIGIN",
                "PRESENT",
                format!("Windows Zone.Identifier origin metadata is present (ZoneId {zone})."),
                Some(format!(
                    "HostUrl: {}; ReferrerUrl: {}",
                    value("HostUrl").as_deref().unwrap_or("unavailable"),
                    value("ReferrerUrl").as_deref().unwrap_or("unavailable")
                )),
            )
        }
        Err(error) if error.kind() == ErrorKind::NotFound => observation(
            "WINDOWS_ORIGIN",
            "NOT_PRESENT",
            "No Windows Zone.Identifier stream was found.".into(),
            None,
        ),
        Err(error) => observation(
            "WINDOWS_ORIGIN",
            "CHECK_FAILED",
            "Windows origin metadata could not be read.".into(),
            Some(error.to_string()),
        ),
    }
}

#[cfg(not(windows))]
fn mark_of_web_observation(_path: &Path) -> TrustObservation {
    observation(
        "WINDOWS_ORIGIN",
        "NOT_APPLICABLE",
        "Windows Mark-of-the-Web evidence is only available on Windows.".into(),
        None,
    )
}

#[cfg(windows)]
fn authenticode_observation(path: &Path) -> TrustObservation {
    let escaped = path.display().to_string().replace('\'', "''");
    let script = format!(
        "$s=Get-AuthenticodeSignature -LiteralPath '{escaped}'; [pscustomobject]@{{Status=$s.Status.ToString();Signer=if($s.SignerCertificate){{$s.SignerCertificate.Subject}}else{{$null}}}} | ConvertTo-Json -Compress"
    );
    match Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
    {
        Ok(output) if output.status.success() => {
            let raw = String::from_utf8_lossy(&output.stdout);
            let value: Value = serde_json::from_str(raw.trim()).unwrap_or(Value::Null);
            let status = value.get("Status").and_then(Value::as_str).unwrap_or("Unknown");
            let signer = value.get("Signer").and_then(Value::as_str).map(ToOwned::to_owned);
            match status {
                "Valid" => observation(
                    "AUTHENTICODE",
                    "VERIFIED",
                    "Windows Authenticode signature is valid.".into(),
                    signer,
                ),
                "NotSigned" => observation(
                    "AUTHENTICODE",
                    "NOT_PRESENT",
                    "The file is not Authenticode-signed.".into(),
                    None,
                ),
                _ => observation(
                    "AUTHENTICODE",
                    "FAILED_VALIDATION",
                    format!("Authenticode status: {status}."),
                    signer,
                ),
            }
        }
        Ok(output) => observation(
            "AUTHENTICODE",
            "CHECK_FAILED",
            "Windows could not evaluate the Authenticode signature.".into(),
            non_empty(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        ),
        Err(error) => observation(
            "AUTHENTICODE",
            "VERIFIER_UNAVAILABLE",
            "PowerShell Authenticode verification is unavailable.".into(),
            Some(error.to_string()),
        ),
    }
}

#[cfg(not(windows))]
fn authenticode_observation(_path: &Path) -> TrustObservation {
    observation(
        "AUTHENTICODE",
        "NOT_APPLICABLE",
        "Authenticode verification is only evaluated on Windows.".into(),
        None,
    )
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then(|| value.chars().take(4000).collect())
}

fn c2pa_observation(path: &Path) -> TrustObservation {
    let output = match Command::new("c2patool").arg(path).arg("--info").output() {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return observation(
                "C2PA",
                "VERIFIER_UNAVAILABLE",
                "Install the official c2patool executable to validate Content Credentials locally.".into(),
                None,
            );
        }
        Err(error) => {
            return observation(
                "C2PA",
                "CHECK_FAILED",
                "OriginKeep could not invoke c2patool.".into(),
                Some(error.to_string()),
            );
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return observation(
            "C2PA",
            "FAILED_OR_UNSUPPORTED",
            "C2PA validation did not complete successfully.".into(),
            non_empty(stderr),
        );
    }
    let lower = stdout.to_ascii_lowercase();
    if lower.contains("no manifest") || lower.contains("no c2pa") || lower.contains("manifest: none") {
        return observation(
            "C2PA",
            "NOT_PRESENT",
            "No C2PA Content Credentials were reported for this asset.".into(),
            non_empty(stdout),
        );
    }
    if lower.contains("valid") && !lower.contains("invalid") && !lower.contains("error") {
        observation(
            "C2PA",
            "MANIFEST_VALIDATED",
            "The official C2PA tool reports a validated Content Credentials manifest.".into(),
            non_empty(stdout),
        )
    } else {
        observation(
            "C2PA",
            "PRESENT_UNVERIFIED",
            "C2PA data was reported, but OriginKeep will not infer publisher identity from ambiguous output.".into(),
            non_empty(stdout),
        )
    }
}

fn sigstore_observation(
    path: &Path,
    expected_identity: Option<&str>,
    expected_issuer: Option<&str>,
) -> TrustObservation {
    let bundle = PathBuf::from(format!("{}.sigstore.json", path.display()));
    if !bundle.is_file() {
        return observation(
            "SIGSTORE",
            "NO_BUNDLE",
            "No adjacent Sigstore verification bundle was found.".into(),
            Some(format!("Expected bundle: {}", bundle.display())),
        );
    }
    let (Some(identity), Some(issuer)) = (expected_identity, expected_issuer) else {
        return observation(
            "SIGSTORE",
            "POLICY_REQUIRED",
            "A Sigstore bundle is present; set the expected signer identity and OIDC issuer before verification.".into(),
            Some(bundle.display().to_string()),
        );
    };
    let output = match Command::new("cosign")
        .args(["verify-blob"])
        .arg(path)
        .args(["--bundle"])
        .arg(&bundle)
        .args([
            "--certificate-identity",
            identity,
            "--certificate-oidc-issuer",
            issuer,
        ])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return observation(
                "SIGSTORE",
                "VERIFIER_UNAVAILABLE",
                "A Sigstore bundle is present, but cosign is not installed.".into(),
                Some(bundle.display().to_string()),
            );
        }
        Err(error) => {
            return observation(
                "SIGSTORE",
                "CHECK_FAILED",
                "OriginKeep could not invoke cosign.".into(),
                Some(error.to_string()),
            );
        }
    };
    if output.status.success() {
        observation(
            "SIGSTORE",
            "VERIFIED_IDENTITY",
            format!("Sigstore verified the blob against expected identity {identity}."),
            Some(format!("issuer: {issuer}; bundle: {}", bundle.display())),
        )
    } else {
        observation(
            "SIGSTORE",
            "FAILED_VALIDATION",
            "Sigstore verification failed for the configured identity policy.".into(),
            non_empty(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        )
    }
}

pub fn refresh_trust(path: &Path, download_id: i64) -> Result<Vec<TrustObservation>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let row = load_passport_row(&connection, download_id)?;
    let passport = assemble_passport(&connection, row)?;
    let file_path = usable_file_path(&passport)
        .ok_or_else(|| "No local file is available for trust inspection".to_string())?;
    let observations = vec![
        integrity_observation(&file_path, passport.sha256.as_deref()),
        mark_of_web_observation(&file_path),
        authenticode_observation(&file_path),
        c2pa_observation(&file_path),
        sigstore_observation(
            &file_path,
            passport.sigstore_identity.as_deref(),
            passport.sigstore_issuer.as_deref(),
        ),
    ];
    for item in observations {
        connection
            .execute(
                r#"
                INSERT INTO trust_observations (
                    download_id, kind, state, summary, details, checked_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP)
                ON CONFLICT(download_id, kind) DO UPDATE SET
                    state = excluded.state,
                    summary = excluded.summary,
                    details = excluded.details,
                    checked_at = CURRENT_TIMESTAMP
                "#,
                params![download_id, item.kind, item.state, item.summary, item.details],
            )
            .map_err(|error| error.to_string())?;
    }
    list_trust(&connection, download_id)
}

pub fn origin_graph(path: &Path) -> Result<OriginGraph, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let rows = {
        let mut statement = connection
            .prepare(
                "SELECT id, file_name, sha256, source_identity, version_number, duplicate_of_id FROM downloads ORDER BY COALESCE(source_identity, ''), COALESCE(version_number, 0), id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_sources = BTreeSet::new();
    let mut seen_content = BTreeSet::new();
    let mut prior_by_source: BTreeMap<String, (i64, String)> = BTreeMap::new();
    for (id, file_name, hash, source, version, duplicate_of) in rows {
        let file_id = format!("file:{id}");
        nodes.push(GraphNode {
            id: file_id.clone(),
            kind: "FILE".into(),
            label: file_name,
            detail: version.map(|value| format!("version {value}")),
        });
        if let Some(source) = source {
            let source_id = format!("source:{source}");
            if seen_sources.insert(source_id.clone()) {
                nodes.push(GraphNode {
                    id: source_id.clone(),
                    kind: "SOURCE".into(),
                    label: source.clone(),
                    detail: None,
                });
            }
            edges.push(GraphEdge {
                from: source_id,
                to: file_id.clone(),
                relation: "PRODUCED".into(),
            });
            if duplicate_of.is_none() {
                if let Some(version) = version {
                    if let Some((previous_version, previous_id)) = prior_by_source.get(&source) {
                        if *previous_version < version {
                            edges.push(GraphEdge {
                                from: previous_id.clone(),
                                to: file_id.clone(),
                                relation: "NEXT_VERSION".into(),
                            });
                        }
                    }
                    prior_by_source.insert(source, (version, file_id.clone()));
                }
            }
        }
        if let Some(hash) = hash {
            let content_id = format!("content:{hash}");
            if seen_content.insert(content_id.clone()) {
                nodes.push(GraphNode {
                    id: content_id.clone(),
                    kind: "CONTENT".into(),
                    label: format!("SHA-256 {}…", &hash[..hash.len().min(12)]),
                    detail: Some(hash.clone()),
                });
                for location in list_locations(&connection, &hash)? {
                    let location_id = format!("location:{}:{}", hash, location.path);
                    nodes.push(GraphNode {
                        id: location_id.clone(),
                        kind: "LOCATION".into(),
                        label: location.path,
                        detail: Some(location.state),
                    });
                    edges.push(GraphEdge {
                        from: content_id.clone(),
                        to: location_id,
                        relation: "LOCATED_AT".into(),
                    });
                }
            }
            edges.push(GraphEdge {
                from: file_id,
                to: content_id,
                relation: if duplicate_of.is_some() {
                    "SAME_CONTENT".into()
                } else {
                    "HAS_CONTENT".into()
                },
            });
        }
    }
    Ok(OriginGraph { nodes, edges })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purposes_are_explicit() {
        assert_eq!(normalize_purpose("Read later").unwrap(), "READ_LATER");
        assert_eq!(normalize_purpose("temporary").unwrap(), "TEMPORARY");
        assert!(normalize_purpose("delete automatically").is_err());
    }

    #[test]
    fn context_deserializes_without_changing_the_capture_contract() {
        let value = serde_json::json!({
            "pageTitle": "Reports",
            "pageUrl": "https://example.com/reports",
            "linkText": "Annual report",
            "contextSource": "enhanced-click"
        });
        let context: CaptureContext = serde_json::from_value(value).unwrap();
        assert_eq!(context.page_title.as_deref(), Some("Reports"));
        assert_eq!(context.link_text.as_deref(), Some("Annual report"));
    }

    #[test]
    fn portable_passport_round_trips() {
        let portable = PortablePassport {
            spec: PASSPORT_SPEC.into(),
            exported_at: "2026-09-05 00:00:00".into(),
            sha256: "abc".into(),
            file_name: "report.pdf".into(),
            mime_type: Some("application/pdf".into()),
            bytes: Some(42),
            original_url: "https://example.com/report.pdf".into(),
            final_url: None,
            referrer: None,
            source_identity: Some("https://example.com/report.pdf".into()),
            downloaded_at: None,
            version_number: Some(1),
            browser_name: Some("Firefox".into()),
            page_title: Some("Reports".into()),
            page_url: Some("https://example.com/reports".into()),
            link_text: Some("Annual report".into()),
            context_text: None,
            context_source: Some("enhanced-click".into()),
            purpose: "REFERENCE".into(),
            note: None,
            expires_at: None,
            trust: Vec::new(),
        };
        let json = serde_json::to_string(&portable).unwrap();
        let decoded: PortablePassport = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.spec, PASSPORT_SPEC);
        assert_eq!(decoded.page_title.as_deref(), Some("Reports"));
    }
}
