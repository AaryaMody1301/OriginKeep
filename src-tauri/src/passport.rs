use crate::{model::DownloadCapture, storage};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const PASSPORT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS file_locations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    download_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    first_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_current INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY(download_id) REFERENCES downloads(id),
    UNIQUE(download_id, path)
);
CREATE INDEX IF NOT EXISTS idx_file_locations_download ON file_locations(download_id, is_current);
CREATE TABLE IF NOT EXISTS passport_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    download_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    details_json TEXT NOT NULL,
    FOREIGN KEY(download_id) REFERENCES downloads(id)
);
CREATE INDEX IF NOT EXISTS idx_passport_events_download ON passport_events(download_id, id DESC);
"#;

const PURPOSES: &[&str] = &[
    "Reference",
    "Read later",
    "Temporary",
    "Work",
    "Receipt",
    "Installer",
    "Dataset",
    "Other",
];
const RETENTION_ACTIONS: &[&str] = &[
    "REVIEW",
    "NEVER_ARCHIVE",
    "ARCHIVE_WHEN_SUPERSEDED",
    "ARCHIVE_AFTER_EXPIRY",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLocation {
    pub path: String,
    pub first_seen: String,
    pub last_seen: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortablePassport {
    pub schema: String,
    pub file_name: String,
    pub sha256: String,
    pub bytes: Option<i64>,
    pub original_url: String,
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub source_identity: Option<String>,
    pub downloaded_at: Option<String>,
    pub version_number: Option<i64>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub link_text: Option<String>,
    pub context_text: Option<String>,
    pub browser_name: Option<String>,
    pub user_note: Option<String>,
    pub purpose: Option<String>,
    pub expires_at: Option<String>,
    pub retention_action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePassport {
    pub download_id: i64,
    pub file_name: String,
    pub local_path: String,
    pub sha256: Option<String>,
    pub bytes: Option<i64>,
    pub original_url: String,
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub source_identity: Option<String>,
    pub status: String,
    pub local_state: String,
    pub version_number: Option<i64>,
    pub duplicate_of_id: Option<i64>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub link_text: Option<String>,
    pub context_text: Option<String>,
    pub browser_name: Option<String>,
    pub user_note: Option<String>,
    pub purpose: Option<String>,
    pub expires_at: Option<String>,
    pub retention_action: String,
    pub locations: Vec<FileLocation>,
    pub os_provenance: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportExport {
    pub download_id: i64,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkResult {
    pub download_id: i64,
    pub old_path: String,
    pub new_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveScanResult {
    pub scanned_files: usize,
    pub matched_files: usize,
    pub relinked: Vec<RelinkResult>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub state: Option<String>,
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

pub fn initialize_database(path: &Path) -> Result<(), String> {
    crate::phase4::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(PASSPORT_SCHEMA)
        .map_err(|error| error.to_string())?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version < 5 {
        connection
            .execute_batch("PRAGMA user_version = 5;")
            .map_err(|error| error.to_string())?;
    }
    backfill_locations(&connection).map_err(|error| error.to_string())
}

fn backfill_locations(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        r#"
        INSERT OR IGNORE INTO file_locations (download_id, path, is_current)
        SELECT id, local_path, CASE WHEN local_state = 'LOCAL_MISSING' THEN 0 ELSE 1 END
        FROM downloads
        WHERE local_path <> '';
        "#,
    )?;
    Ok(())
}

pub fn list_passports(path: &Path) -> Result<Vec<FilePassport>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let ids = {
        let mut statement = connection
            .prepare("SELECT id FROM downloads ORDER BY updated_at DESC, id DESC")
            .map_err(|error| error.to_string())?;
        statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };
    ids.into_iter()
        .map(|id| passport_with_connection(&connection, id).map_err(|error| error.to_string()))
        .collect()
}

pub fn get_passport(path: &Path, download_id: i64) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    passport_with_connection(&connection, download_id).map_err(|error| error.to_string())
}

fn passport_with_connection(
    connection: &Connection,
    download_id: i64,
) -> rusqlite::Result<FilePassport> {
    let mut passport = connection.query_row(
        r#"
        SELECT id, file_name, local_path, sha256, bytes, original_url, final_url, referrer,
               source_identity, status, local_state, version_number, duplicate_of_id,
               page_title, page_url, link_text, context_text, browser_name,
               user_note, purpose, expires_at, retention_action
        FROM downloads WHERE id = ?1
        "#,
        [download_id],
        |row| {
            Ok(FilePassport {
                download_id: row.get(0)?,
                file_name: row.get(1)?,
                local_path: row.get(2)?,
                sha256: row.get(3)?,
                bytes: row.get(4)?,
                original_url: row.get(5)?,
                final_url: row.get(6)?,
                referrer: row.get(7)?,
                source_identity: row.get(8)?,
                status: row.get(9)?,
                local_state: row.get(10)?,
                version_number: row.get(11)?,
                duplicate_of_id: row.get(12)?,
                page_title: row.get(13)?,
                page_url: row.get(14)?,
                link_text: row.get(15)?,
                context_text: row.get(16)?,
                browser_name: row.get(17)?,
                user_note: row.get(18)?,
                purpose: row.get(19)?,
                expires_at: row.get(20)?,
                retention_action: row.get(21)?,
                locations: Vec::new(),
                os_provenance: None,
            })
        },
    )?;

    let mut location_statement = connection.prepare(
        "SELECT path, first_seen, last_seen, is_current FROM file_locations WHERE download_id = ?1 ORDER BY is_current DESC, last_seen DESC",
    )?;
    passport.locations = location_statement
        .query_map([download_id], |row| {
            Ok(FileLocation {
                path: row.get(0)?,
                first_seen: row.get(1)?,
                last_seen: row.get(2)?,
                is_current: row.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    passport.os_provenance = connection
        .query_row(
            "SELECT details_json FROM passport_events WHERE download_id = ?1 AND event_type = 'OS_PROVENANCE' ORDER BY id DESC LIMIT 1",
            [download_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(passport)
}

pub fn update_metadata(
    path: &Path,
    download_id: i64,
    user_note: Option<String>,
    purpose: Option<String>,
    expires_at: Option<String>,
    retention_action: String,
) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let purpose = clean_optional(purpose, 80)?;
    if let Some(value) = purpose.as_deref() {
        if !PURPOSES.contains(&value) {
            return Err(format!("Unsupported purpose: {value}"));
        }
    }
    if !RETENTION_ACTIONS.contains(&retention_action.as_str()) {
        return Err(format!("Unsupported retention action: {retention_action}"));
    }
    let note = clean_optional(user_note, 4_000)?;
    let expiry = clean_optional(expires_at, 64)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let updated = connection
        .execute(
            r#"
            UPDATE downloads
            SET user_note = ?1, purpose = ?2, expires_at = ?3, retention_action = ?4,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?5
            "#,
            params![note, purpose, expiry, retention_action, download_id],
        )
        .map_err(|error| error.to_string())?;
    if updated == 0 {
        return Err(format!("Download record #{download_id} does not exist"));
    }
    record_event(
        &connection,
        download_id,
        "INTENT_UPDATED",
        &serde_json::json!({"purpose": purpose, "expiresAt": expiry, "retentionAction": retention_action}).to_string(),
    )
    .map_err(|error| error.to_string())?;
    passport_with_connection(&connection, download_id).map_err(|error| error.to_string())
}

fn clean_optional(value: Option<String>, max: usize) -> Result<Option<String>, String> {
    match value {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else if trimmed.chars().count() > max {
                Err(format!("Value exceeds the {max}-character limit"))
            } else {
                Ok(Some(trimmed.to_string()))
            }
        }
        None => Ok(None),
    }
}

pub fn export_passport(path: &Path, download_id: i64) -> Result<PassportExport, String> {
    let passport = get_passport(path, download_id)?;
    let hash = passport
        .sha256
        .clone()
        .ok_or_else(|| "A portable passport requires a recorded SHA-256 fingerprint".to_string())?;
    let local = PathBuf::from(&passport.local_path);
    if !local.is_file() {
        return Err("The local file must be present before exporting its passport".into());
    }
    let current = storage::sha256_file(&local).map_err(|error| error.to_string())?;
    if current != hash {
        return Err("Passport export blocked because local bytes no longer match the recorded SHA-256".into());
    }
    let portable = PortablePassport {
        schema: "https://originkeep.local/passport/v1".into(),
        file_name: passport.file_name.clone(),
        sha256: hash.clone(),
        bytes: passport.bytes,
        original_url: passport.original_url,
        final_url: passport.final_url,
        referrer: passport.referrer,
        source_identity: passport.source_identity,
        downloaded_at: None,
        version_number: passport.version_number,
        page_title: passport.page_title,
        page_url: passport.page_url,
        link_text: passport.link_text,
        context_text: passport.context_text,
        browser_name: passport.browser_name,
        user_note: passport.user_note,
        purpose: passport.purpose,
        expires_at: passport.expires_at,
        retention_action: passport.retention_action,
    };
    let sidecar = PathBuf::from(format!("{}.originkeep.json", local.display()));
    let json = serde_json::to_string_pretty(&portable).map_err(|error| error.to_string())?;
    fs::write(&sidecar, json).map_err(|error| error.to_string())?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let _ = record_event(
        &connection,
        download_id,
        "PASSPORT_EXPORTED",
        &serde_json::json!({"sidecarFile": sidecar.file_name().map(|v| v.to_string_lossy())}).to_string(),
    );
    Ok(PassportExport {
        download_id,
        path: sidecar.display().to_string(),
        sha256: hash,
    })
}

pub fn import_passport(path: &Path, passport_path: String) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let sidecar = PathBuf::from(passport_path);
    let json = fs::read_to_string(&sidecar).map_err(|error| error.to_string())?;
    let portable: PortablePassport = serde_json::from_str(&json).map_err(|error| error.to_string())?;
    if portable.schema != "https://originkeep.local/passport/v1" {
        return Err("Unsupported OriginKeep Passport schema".into());
    }
    let sidecar_text = sidecar.to_string_lossy();
    let asset_text = sidecar_text
        .strip_suffix(".originkeep.json")
        .ok_or_else(|| "Passport file name must end with .originkeep.json".to_string())?;
    let asset = PathBuf::from(asset_text);
    if !asset.is_file() {
        return Err(format!("Passport asset is missing: {}", asset.display()));
    }
    let current = storage::sha256_file(&asset).map_err(|error| error.to_string())?;
    if current != portable.sha256 {
        return Err("Passport import blocked because the adjacent file does not match the passport SHA-256".into());
    }
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let capture = DownloadCapture {
        capture_key: format!("passport:{}:{unique}", portable.sha256),
        browser_download_id: 0,
        original_url: portable.original_url.clone(),
        final_url: portable.final_url.clone(),
        referrer: portable.referrer.clone(),
        local_path: asset.display().to_string(),
        file_name: asset
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| portable.file_name.clone()),
        mime_type: None,
        bytes: fs::metadata(&asset).ok().map(|metadata| metadata.len() as i64),
        started_at: portable.downloaded_at.clone(),
        completed_at: portable.downloaded_at.clone(),
        state: "complete".into(),
        page_title: portable.page_title.clone(),
        page_url: portable.page_url.clone(),
        link_text: portable.link_text.clone(),
        context_text: portable.context_text.clone(),
        browser_name: Some("OriginKeep Passport import".into()),
    };
    let result = storage::ingest_capture(path, &capture)?;
    update_metadata(
        path,
        result.id,
        portable.user_note,
        portable.purpose,
        portable.expires_at,
        portable.retention_action,
    )
}

pub fn relink_file(path: &Path, download_id: i64, new_path: String) -> Result<RelinkResult, String> {
    initialize_database(path)?;
    let candidate = PathBuf::from(&new_path);
    if !candidate.is_file() {
        return Err("The new location is not a regular file".into());
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let (old_path, expected): (String, Option<String>) = connection
        .query_row(
            "SELECT local_path, sha256 FROM downloads WHERE id = ?1",
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Download record #{download_id} does not exist"))?;
    let expected = expected.ok_or_else(|| "Relinking requires a recorded SHA-256".to_string())?;
    let current = storage::sha256_file(&candidate).map_err(|error| error.to_string())?;
    if current != expected {
        return Err("Relink blocked: candidate bytes do not match the recorded file identity".into());
    }
    let file_name = candidate
        .file_name()
        .ok_or_else(|| "Candidate path has no file name".to_string())?
        .to_string_lossy()
        .into_owned();
    connection
        .execute_batch("BEGIN IMMEDIATE TRANSACTION;")
        .map_err(|error| error.to_string())?;
    let result = (|| -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE file_locations SET is_current = 0, last_seen = CURRENT_TIMESTAMP WHERE download_id = ?1",
            [download_id],
        )?;
        connection.execute(
            r#"
            INSERT INTO file_locations (download_id, path, is_current)
            VALUES (?1, ?2, 1)
            ON CONFLICT(download_id, path) DO UPDATE SET
                is_current = 1, last_seen = CURRENT_TIMESTAMP
            "#,
            params![download_id, new_path],
        )?;
        connection.execute(
            "UPDATE downloads SET local_path = ?1, file_name = ?2, local_state = 'PRESENT', updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![new_path, file_name, download_id],
        )?;
        record_event(
            &connection,
            download_id,
            "FILE_RELINKED",
            &serde_json::json!({"from": old_path, "to": new_path}).to_string(),
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => connection
            .execute_batch("COMMIT;")
            .map_err(|error| error.to_string())?,
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            return Err(error.to_string());
        }
    }
    Ok(RelinkResult {
        download_id,
        old_path,
        new_path,
        sha256: expected,
    })
}

pub fn scan_for_moves(path: &Path, root: String, max_files: usize) -> Result<MoveScanResult, String> {
    initialize_database(path)?;
    let root = PathBuf::from(root);
    if !root.is_dir() {
        return Err("Move scan root must be an existing directory".into());
    }
    let max_files = max_files.clamp(1, 50_000);
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let missing = {
        let mut statement = connection
            .prepare("SELECT id, sha256, bytes FROM downloads WHERE local_state = 'LOCAL_MISSING' AND sha256 IS NOT NULL")
            .map_err(|error| error.to_string())?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };
    let mut stack = vec![root];
    let mut scanned = 0usize;
    let mut relinked = Vec::new();
    let mut truncated = false;
    while let Some(directory) = stack.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate.is_dir() {
                stack.push(candidate);
                continue;
            }
            if !candidate.is_file() || candidate.to_string_lossy().ends_with(".originkeep.json") {
                continue;
            }
            scanned += 1;
            if scanned > max_files {
                truncated = true;
                break;
            }
            let length = fs::metadata(&candidate).ok().map(|metadata| metadata.len() as i64);
            let possible = missing.iter().filter(|(id, _, bytes)| {
                !relinked.iter().any(|result: &RelinkResult| result.download_id == *id)
                    && (bytes.is_none() || *bytes == length)
            });
            let mut hash: Option<String> = None;
            for (id, expected, _) in possible {
                let current = match hash.as_ref() {
                    Some(value) => value.clone(),
                    None => {
                        let value = match storage::sha256_file(&candidate) {
                            Ok(value) => value,
                            Err(_) => break,
                        };
                        hash = Some(value.clone());
                        value
                    }
                };
                if &current == expected {
                    if let Ok(result) = relink_file(path, *id, candidate.display().to_string()) {
                        relinked.push(result);
                    }
                    break;
                }
            }
        }
        if truncated {
            break;
        }
    }
    Ok(MoveScanResult {
        scanned_files: scanned.min(max_files),
        matched_files: relinked.len(),
        relinked,
        truncated,
    })
}

pub fn import_os_provenance(path: &Path, download_id: i64) -> Result<FilePassport, String> {
    initialize_database(path)?;
    let passport = get_passport(path, download_id)?;
    let local = PathBuf::from(&passport.local_path);
    let evidence = os_provenance_for(&local)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    record_event(
        &connection,
        download_id,
        "OS_PROVENANCE",
        &serde_json::json!({"platform": std::env::consts::OS, "evidence": evidence}).to_string(),
    )
    .map_err(|error| error.to_string())?;
    passport_with_connection(&connection, download_id).map_err(|error| error.to_string())
}

fn os_provenance_for(local: &Path) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let ads = PathBuf::from(format!("{}:Zone.Identifier", local.display()));
        return match fs::read_to_string(ads) {
            Ok(value) => Ok(value),
            Err(_) => Ok("No readable Windows Zone.Identifier metadata was found.".into()),
        };
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("mdls")
            .args(["-raw", "-name", "kMDItemWhereFroms"])
            .arg(local)
            .output()
            .map_err(|error| error.to_string())?;
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("gio")
            .args(["info", "--attributes=metadata::download-uri"])
            .arg(local)
            .output();
        return match output {
            Ok(output) if output.status.success() => {
                Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => Ok("No portable Linux download-origin metadata was available; browser Passport evidence remains authoritative.".into()),
        };
    }
    #[allow(unreachable_code)]
    Ok("OS provenance import is unavailable on this platform.".into())
}

pub fn origin_graph(path: &Path) -> Result<OriginGraph, String> {
    let passports = list_passports(path)?;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut sources = std::collections::BTreeSet::new();
    for passport in &passports {
        let file_id = format!("file:{}", passport.download_id);
        nodes.push(GraphNode {
            id: file_id.clone(),
            kind: "FILE".into(),
            label: passport.file_name.clone(),
            state: Some(format!("{} / {}", passport.status, passport.local_state)),
        });
        if let Some(source) = passport.source_identity.as_ref() {
            let source_id = format!("source:{source}");
            if sources.insert(source.clone()) {
                nodes.push(GraphNode {
                    id: source_id.clone(),
                    kind: "SOURCE".into(),
                    label: source.clone(),
                    state: None,
                });
            }
            edges.push(GraphEdge {
                from: source_id,
                to: file_id.clone(),
                relation: "ORIGIN".into(),
            });
        }
        if let Some(parent) = passport.duplicate_of_id {
            edges.push(GraphEdge {
                from: format!("file:{parent}"),
                to: file_id.clone(),
                relation: "EXACT_DUPLICATE".into(),
            });
        }
    }
    for passport in &passports {
        let (Some(source), Some(version)) = (&passport.source_identity, passport.version_number) else {
            continue;
        };
        if version <= 1 || passport.duplicate_of_id.is_some() {
            continue;
        }
        if let Some(previous) = passports.iter().find(|candidate| {
            candidate.source_identity.as_ref() == Some(source)
                && candidate.version_number == Some(version - 1)
                && candidate.duplicate_of_id.is_none()
        }) {
            edges.push(GraphEdge {
                from: format!("file:{}", previous.download_id),
                to: format!("file:{}", passport.download_id),
                relation: "NEXT_VERSION".into(),
            });
        }
    }
    Ok(OriginGraph { nodes, edges })
}

fn record_event(
    connection: &Connection,
    download_id: i64,
    event_type: &str,
    details_json: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO passport_events (download_id, event_type, details_json) VALUES (?1, ?2, ?3)",
        params![download_id, event_type, details_json],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn unique_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("originkeep-passport-{unique}-{name}"))
    }

    #[test]
    fn portable_passport_does_not_serialize_local_paths() {
        let passport = PortablePassport {
            schema: "https://originkeep.local/passport/v1".into(),
            file_name: "report.pdf".into(),
            sha256: "a".repeat(64),
            bytes: Some(10),
            original_url: "https://example.com/report.pdf".into(),
            final_url: None,
            referrer: None,
            source_identity: Some("https://example.com/report.pdf".into()),
            downloaded_at: None,
            version_number: Some(1),
            page_title: None,
            page_url: None,
            link_text: None,
            context_text: None,
            browser_name: None,
            user_note: None,
            purpose: None,
            expires_at: None,
            retention_action: "REVIEW".into(),
        };
        let json = serde_json::to_string(&passport).unwrap();
        assert!(!json.contains("localPath"));
        assert!(!json.contains("file_locations"));
    }

    #[test]
    fn relink_requires_identical_bytes() {
        let database = unique_path("db.sqlite");
        let original = unique_path("original.txt");
        let moved = unique_path("moved.txt");
        fs::write(&original, b"same bytes").unwrap();
        fs::write(&moved, b"same bytes").unwrap();
        let capture = DownloadCapture {
            capture_key: "passport-relink-test".into(),
            browser_download_id: 1,
            original_url: "https://example.com/file.txt".into(),
            final_url: None,
            referrer: None,
            local_path: original.display().to_string(),
            file_name: "original.txt".into(),
            mime_type: Some("text/plain".into()),
            bytes: Some(10),
            started_at: None,
            completed_at: None,
            state: "complete".into(),
            page_title: None,
            page_url: None,
            link_text: None,
            context_text: None,
            browser_name: None,
        };
        let result = storage::ingest_capture(&database, &capture).unwrap();
        initialize_database(&database).unwrap();
        let relinked = relink_file(&database, result.id, moved.display().to_string()).unwrap();
        assert_eq!(relinked.new_path, moved.display().to_string());
        let passport = get_passport(&database, result.id).unwrap();
        assert_eq!(passport.locations.len(), 2);
        fs::remove_file(database).ok();
        fs::remove_file(original).ok();
        fs::remove_file(moved).ok();
    }
}
