use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

const PHASE4_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS lifecycle_entries (
    download_id INTEGER PRIMARY KEY,
    original_path TEXT NOT NULL,
    archive_path TEXT,
    state TEXT NOT NULL DEFAULT 'ACTIVE',
    archived_at TEXT,
    restored_at TEXT,
    last_error TEXT,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(download_id) REFERENCES downloads(id)
);
CREATE INDEX IF NOT EXISTS idx_lifecycle_entries_state ON lifecycle_entries(state);
"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleItem {
    pub download_id: i64,
    pub file_name: String,
    pub original_path: String,
    pub bytes: Option<i64>,
    pub status: String,
    pub local_state: String,
    pub source_identity: Option<String>,
    pub version_number: Option<i64>,
    pub duplicate_of_id: Option<i64>,
    pub lifecycle_state: String,
    pub archive_path: Option<String>,
    pub reclaimable: bool,
    pub archive_eligible: bool,
    pub restore_eligible: bool,
    pub recommendation: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleSummary {
    pub tracked_bytes: i64,
    pub present_bytes: i64,
    pub archived_bytes: i64,
    pub reclaimable_bytes: i64,
    pub duplicate_bytes: i64,
    pub superseded_bytes: i64,
    pub protected_bytes: i64,
    pub candidate_count: i64,
    pub archived_count: i64,
    pub database_health: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleReview {
    pub keep_latest_versions: i64,
    pub include_duplicates: bool,
    pub summary: LifecycleSummary,
    pub items: Vec<LifecycleItem>,
}

#[derive(Debug)]
struct StoredDownload {
    id: i64,
    file_name: String,
    local_path: String,
    bytes: Option<i64>,
    status: String,
    local_state: String,
    source_identity: Option<String>,
    version_number: Option<i64>,
    duplicate_of_id: Option<i64>,
    sha256: Option<String>,
    max_family_version: Option<i64>,
    lifecycle_state: String,
    archive_path: Option<String>,
}

pub fn initialize_database(path: &Path) -> Result<(), String> {
    crate::phase3::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())?;
    reconcile_incomplete_operations(&connection).map_err(|error| error.to_string())?;
    Ok(())
}

fn initialize_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(PHASE4_SCHEMA)?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 4 {
        connection.execute_batch("PRAGMA user_version = 4;")?;
    }
    Ok(())
}

pub fn lifecycle_review(
    path: &Path,
    keep_latest_versions: i64,
    include_duplicates: bool,
) -> Result<LifecycleReview, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let keep_latest_versions = keep_latest_versions.clamp(1, 20);
    build_review(&connection, keep_latest_versions, include_duplicates)
        .map_err(|error| error.to_string())
}

fn build_review(
    connection: &Connection,
    keep_latest_versions: i64,
    include_duplicates: bool,
) -> rusqlite::Result<LifecycleReview> {
    initialize_connection(connection)?;
    let downloads = load_downloads(connection)?;
    let mut items = Vec::with_capacity(downloads.len());
    let mut summary = LifecycleSummary {
        tracked_bytes: 0,
        present_bytes: 0,
        archived_bytes: 0,
        reclaimable_bytes: 0,
        duplicate_bytes: 0,
        superseded_bytes: 0,
        protected_bytes: 0,
        candidate_count: 0,
        archived_count: 0,
        database_health: database_health(connection)?,
    };

    for download in downloads {
        let item = classify_download(&download, keep_latest_versions, include_duplicates);
        let bytes = download.bytes.unwrap_or(0).max(0);
        summary.tracked_bytes += bytes;
        if item.lifecycle_state == "ARCHIVED" {
            summary.archived_bytes += bytes;
            summary.archived_count += 1;
        } else if download.local_state == "PRESENT" {
            summary.present_bytes += bytes;
        }
        if download.duplicate_of_id.is_some() && download.local_state == "PRESENT" {
            summary.duplicate_bytes += bytes;
        }
        if download.status == "SUPERSEDED" && download.local_state == "PRESENT" {
            summary.superseded_bytes += bytes;
        }
        if item.reclaimable {
            summary.reclaimable_bytes += bytes;
            summary.candidate_count += 1;
        } else if download.local_state == "PRESENT" {
            summary.protected_bytes += bytes;
        }
        items.push(item);
    }

    Ok(LifecycleReview {
        keep_latest_versions,
        include_duplicates,
        summary,
        items,
    })
}

fn load_downloads(connection: &Connection) -> rusqlite::Result<Vec<StoredDownload>> {
    let mut statement = connection.prepare(
        r#"
        SELECT d.id, d.file_name, d.local_path, d.bytes, d.status, d.local_state,
               d.source_identity, d.version_number, d.duplicate_of_id, d.sha256,
               CASE WHEN d.source_identity IS NULL THEN NULL ELSE (
                   SELECT MAX(v.version_number)
                   FROM downloads v
                   WHERE v.source_identity = d.source_identity
                     AND v.duplicate_of_id IS NULL
               ) END AS max_family_version,
               COALESCE(l.state, 'ACTIVE'), l.archive_path
        FROM downloads d
        LEFT JOIN lifecycle_entries l ON l.download_id = d.id
        ORDER BY d.updated_at DESC, d.id DESC
        "#,
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(StoredDownload {
                id: row.get(0)?,
                file_name: row.get(1)?,
                local_path: row.get(2)?,
                bytes: row.get(3)?,
                status: row.get(4)?,
                local_state: row.get(5)?,
                source_identity: row.get(6)?,
                version_number: row.get(7)?,
                duplicate_of_id: row.get(8)?,
                sha256: row.get(9)?,
                max_family_version: row.get(10)?,
                lifecycle_state: row.get(11)?,
                archive_path: row.get(12)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn classify_download(
    download: &StoredDownload,
    keep_latest_versions: i64,
    include_duplicates: bool,
) -> LifecycleItem {
    let archived = download.lifecycle_state == "ARCHIVED" || download.local_state == "ARCHIVED";
    let local_file = Path::new(&download.local_path);
    let archive_eligible = !archived
        && download.local_state == "PRESENT"
        && download.sha256.is_some()
        && local_file.is_file();
    let restore_eligible = archived
        && download
            .archive_path
            .as_deref()
            .is_some_and(|value| Path::new(value).is_file());

    let (reclaimable, recommendation, reason) = if archived {
        (
            false,
            "ARCHIVED".to_string(),
            "The local copy is stored in OriginKeep's recoverable archive.".to_string(),
        )
    } else if download.local_state == "LOCAL_MODIFIED" {
        (
            false,
            "PROTECT".to_string(),
            "Local bytes no longer match the recorded download fingerprint; cleanup is blocked."
                .to_string(),
        )
    } else if download.local_state == "LOCAL_MISSING" {
        (
            false,
            "MISSING".to_string(),
            "The original local path is already missing; no cleanup action is proposed."
                .to_string(),
        )
    } else if download.sha256.is_none() {
        (
            false,
            "PROTECT".to_string(),
            "No stored SHA-256 exists, so OriginKeep cannot prove the file is unchanged before cleanup.".to_string(),
        )
    } else if include_duplicates && download.duplicate_of_id.is_some() && archive_eligible {
        (
            true,
            "ARCHIVE_CANDIDATE".to_string(),
            format!(
                "Exact SHA-256 duplicate of record #{}; this copy can be archived without losing unique content.",
                download.duplicate_of_id.unwrap_or_default()
            ),
        )
    } else if archive_eligible && outside_retention_window(download, keep_latest_versions) {
        (
            true,
            "ARCHIVE_CANDIDATE".to_string(),
            format!(
                "Version v{} is outside the keep-latest-{} policy; v{} is the newest primary version in this source family.",
                download.version_number.unwrap_or_default(),
                keep_latest_versions,
                download.max_family_version.unwrap_or_default()
            ),
        )
    } else {
        (
            false,
            "KEEP".to_string(),
            "No deterministic cleanup rule selects this file under the current retention policy."
                .to_string(),
        )
    };

    LifecycleItem {
        download_id: download.id,
        file_name: download.file_name.clone(),
        original_path: download.local_path.clone(),
        bytes: download.bytes,
        status: download.status.clone(),
        local_state: download.local_state.clone(),
        source_identity: download.source_identity.clone(),
        version_number: download.version_number,
        duplicate_of_id: download.duplicate_of_id,
        lifecycle_state: download.lifecycle_state.clone(),
        archive_path: download.archive_path.clone(),
        reclaimable,
        archive_eligible,
        restore_eligible,
        recommendation,
        reason,
    }
}

fn outside_retention_window(download: &StoredDownload, keep_latest_versions: i64) -> bool {
    let Some(version) = download.version_number else {
        return false;
    };
    let Some(max_version) = download.max_family_version else {
        return false;
    };
    download.duplicate_of_id.is_none()
        && download.status == "SUPERSEDED"
        && version <= max_version.saturating_sub(keep_latest_versions)
}

pub fn archive_download(path: &Path, download_id: i64) -> Result<LifecycleItem, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let download = load_download(&connection, download_id)?;

    if download.lifecycle_state == "ARCHIVED" || download.local_state == "ARCHIVED" {
        return Err("This download is already in the recoverable archive".into());
    }
    if download.local_state != "PRESENT" {
        return Err(format!(
            "Safe archive requires PRESENT local state; this record is {}",
            download.local_state
        ));
    }
    let expected_hash = download
        .sha256
        .as_deref()
        .ok_or_else(|| "Safe archive requires a stored SHA-256 fingerprint".to_string())?;
    let original_path = PathBuf::from(&download.local_path);
    if !original_path.is_file() {
        return Err("The original file is missing; verify local files before archiving".into());
    }
    let current_hash = crate::storage::sha256_file(&original_path)
        .map_err(|error| format!("Could not hash the original file: {error}"))?;
    if current_hash != expected_hash {
        connection
            .execute(
                "UPDATE downloads SET local_state = 'LOCAL_MODIFIED', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                [download_id],
            )
            .map_err(|error| error.to_string())?;
        return Err(
            "Archive blocked because the local bytes differ from the recorded SHA-256".into(),
        );
    }

    let archive_path = archive_path_for(path, &download.file_name, download_id, expected_hash)?;
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if archive_path.exists() {
        let existing_hash = crate::storage::sha256_file(&archive_path)
            .map_err(|error| format!("Could not verify the existing archive file: {error}"))?;
        if existing_hash != expected_hash {
            return Err(
                "Archive collision: an existing archive path contains different bytes".into(),
            );
        }
        fs::remove_file(&archive_path).map_err(|error| error.to_string())?;
    }

    connection
        .execute(
            r#"
            INSERT INTO lifecycle_entries (download_id, original_path, archive_path, state, last_error)
            VALUES (?1, ?2, ?3, 'ARCHIVING', NULL)
            ON CONFLICT(download_id) DO UPDATE SET
                original_path = excluded.original_path,
                archive_path = excluded.archive_path,
                state = 'ARCHIVING',
                last_error = NULL,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![download_id, download.local_path, archive_path.display().to_string()],
        )
        .map_err(|error| error.to_string())?;

    if let Err(error) = copy_and_verify(&original_path, &archive_path, expected_hash) {
        mark_lifecycle_error(&connection, download_id, "ACTIVE", &error)?;
        let _ = fs::remove_file(&archive_path);
        return Err(error);
    }
    if let Err(error) = fs::remove_file(&original_path) {
        let message = format!("Copied archive bytes but could not remove the original: {error}");
        let _ = fs::remove_file(&archive_path);
        mark_lifecycle_error(&connection, download_id, "ACTIVE", &message)?;
        return Err(message);
    }

    connection
        .execute_batch("BEGIN IMMEDIATE TRANSACTION;")
        .map_err(|error| error.to_string())?;
    let result = (|| -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE downloads SET local_state = 'ARCHIVED', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [download_id],
        )?;
        connection.execute(
            r#"
            UPDATE lifecycle_entries
            SET state = 'ARCHIVED', archived_at = CURRENT_TIMESTAMP,
                last_error = NULL, updated_at = CURRENT_TIMESTAMP
            WHERE download_id = ?1
            "#,
            [download_id],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => connection
            .execute_batch("COMMIT;")
            .map_err(|error| error.to_string())?,
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            return Err(format!(
                "The file was archived but metadata finalization failed; startup recovery will reconcile it: {error}"
            ));
        }
    }

    lifecycle_item(&connection, download_id, 1, true)
}

pub fn restore_download(path: &Path, download_id: i64) -> Result<LifecycleItem, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let download = load_download(&connection, download_id)?;
    if download.lifecycle_state != "ARCHIVED" && download.local_state != "ARCHIVED" {
        return Err("Restore is available only for files in the recoverable archive".into());
    }
    let expected_hash = download
        .sha256
        .as_deref()
        .ok_or_else(|| "Restore requires the original SHA-256 fingerprint".to_string())?;
    let archive_path = download
        .archive_path
        .as_deref()
        .map(PathBuf::from)
        .ok_or_else(|| "The archive metadata has no archive path".to_string())?;
    if !archive_path.is_file() {
        return Err("The recoverable archive copy is missing".into());
    }
    let archive_hash = crate::storage::sha256_file(&archive_path)
        .map_err(|error| format!("Could not verify the archive copy: {error}"))?;
    if archive_hash != expected_hash {
        return Err(
            "Restore blocked because the archive bytes do not match the recorded SHA-256".into(),
        );
    }

    let original_path = PathBuf::from(&download.local_path);
    if original_path.exists() {
        if !original_path.is_file() {
            return Err("Restore collision: the original path now exists but is not a file".into());
        }
        let existing_hash = crate::storage::sha256_file(&original_path)
            .map_err(|error| format!("Could not inspect the restore collision: {error}"))?;
        if existing_hash != expected_hash {
            return Err("Restore refused to overwrite different bytes at the original path".into());
        }
    }
    if let Some(parent) = original_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    connection
        .execute(
            "UPDATE lifecycle_entries SET state = 'RESTORING', last_error = NULL, updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
            [download_id],
        )
        .map_err(|error| error.to_string())?;

    if !original_path.exists() {
        if let Err(error) = copy_and_verify(&archive_path, &original_path, expected_hash) {
            mark_lifecycle_error(&connection, download_id, "ARCHIVED", &error)?;
            let _ = fs::remove_file(&original_path);
            return Err(error);
        }
    }
    if let Err(error) = fs::remove_file(&archive_path) {
        let message =
            format!("Restored the original bytes but could not remove the archive copy: {error}");
        mark_lifecycle_error(&connection, download_id, "RESTORING", &message)?;
        return Err(message);
    }

    connection
        .execute_batch("BEGIN IMMEDIATE TRANSACTION;")
        .map_err(|error| error.to_string())?;
    let result = (|| -> rusqlite::Result<()> {
        connection.execute(
            "UPDATE downloads SET local_state = 'PRESENT', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [download_id],
        )?;
        connection.execute(
            r#"
            UPDATE lifecycle_entries
            SET state = 'ACTIVE', restored_at = CURRENT_TIMESTAMP,
                last_error = NULL, updated_at = CURRENT_TIMESTAMP
            WHERE download_id = ?1
            "#,
            [download_id],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => connection
            .execute_batch("COMMIT;")
            .map_err(|error| error.to_string())?,
        Err(error) => {
            let _ = connection.execute_batch("ROLLBACK;");
            return Err(format!(
                "The file was restored but metadata finalization failed; startup recovery will reconcile it: {error}"
            ));
        }
    }

    lifecycle_item(&connection, download_id, 1, true)
}

fn load_download(connection: &Connection, download_id: i64) -> Result<StoredDownload, String> {
    initialize_connection(connection).map_err(|error| error.to_string())?;
    connection
        .query_row(
            r#"
            SELECT d.id, d.file_name, d.local_path, d.bytes, d.status, d.local_state,
                   d.source_identity, d.version_number, d.duplicate_of_id, d.sha256,
                   CASE WHEN d.source_identity IS NULL THEN NULL ELSE (
                       SELECT MAX(v.version_number) FROM downloads v
                       WHERE v.source_identity = d.source_identity AND v.duplicate_of_id IS NULL
                   ) END,
                   COALESCE(l.state, 'ACTIVE'), l.archive_path
            FROM downloads d
            LEFT JOIN lifecycle_entries l ON l.download_id = d.id
            WHERE d.id = ?1
            "#,
            [download_id],
            |row| {
                Ok(StoredDownload {
                    id: row.get(0)?,
                    file_name: row.get(1)?,
                    local_path: row.get(2)?,
                    bytes: row.get(3)?,
                    status: row.get(4)?,
                    local_state: row.get(5)?,
                    source_identity: row.get(6)?,
                    version_number: row.get(7)?,
                    duplicate_of_id: row.get(8)?,
                    sha256: row.get(9)?,
                    max_family_version: row.get(10)?,
                    lifecycle_state: row.get(11)?,
                    archive_path: row.get(12)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Download record #{download_id} does not exist"))
}

fn lifecycle_item(
    connection: &Connection,
    download_id: i64,
    keep_latest_versions: i64,
    include_duplicates: bool,
) -> Result<LifecycleItem, String> {
    let download = load_download(connection, download_id)?;
    Ok(classify_download(
        &download,
        keep_latest_versions,
        include_duplicates,
    ))
}

fn archive_path_for(
    database_path: &Path,
    file_name: &str,
    download_id: i64,
    hash: &str,
) -> Result<PathBuf, String> {
    let parent = database_path
        .parent()
        .ok_or_else(|| "Database path has no application-data parent".to_string())?;
    let short_hash: String = hash.chars().take(12).collect();
    Ok(parent.join("archive").join(format!(
        "{download_id}-{short_hash}-{}",
        safe_file_name(file_name)
    )))
}

fn safe_file_name(file_name: &str) -> String {
    let cleaned: String = file_name
        .chars()
        .map(|character| match character {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            value if value.is_control() => '_',
            value => value,
        })
        .collect();
    if cleaned.trim().is_empty() {
        "download.bin".into()
    } else {
        cleaned
    }
}

fn copy_and_verify(source: &Path, destination: &Path, expected_hash: &str) -> Result<(), String> {
    fs::copy(source, destination).map_err(|error| {
        format!(
            "Could not copy {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })?;
    File::open(destination)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("Could not flush the copied file: {error}"))?;
    let copied_hash = crate::storage::sha256_file(destination)
        .map_err(|error| format!("Could not verify copied bytes: {error}"))?;
    if copied_hash != expected_hash {
        return Err("Copied bytes failed SHA-256 verification".into());
    }
    Ok(())
}

fn mark_lifecycle_error(
    connection: &Connection,
    download_id: i64,
    state: &str,
    message: &str,
) -> Result<(), String> {
    connection
        .execute(
            "UPDATE lifecycle_entries SET state = ?1, last_error = ?2, updated_at = CURRENT_TIMESTAMP WHERE download_id = ?3",
            params![state, message, download_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn reconcile_incomplete_operations(connection: &Connection) -> rusqlite::Result<()> {
    let pending = {
        let mut statement = connection.prepare(
            r#"
            SELECT l.download_id, l.original_path, l.archive_path, l.state, d.sha256
            FROM lifecycle_entries l
            JOIN downloads d ON d.id = l.download_id
            WHERE l.state IN ('ARCHIVING', 'RESTORING')
            "#,
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    for (download_id, original_path, archive_path, state, expected_hash) in pending {
        let original = PathBuf::from(original_path);
        let archive = archive_path.as_deref().map(PathBuf::from);
        let original_valid = expected_hash
            .as_deref()
            .is_some_and(|hash| file_matches(&original, hash));
        let archive_valid = expected_hash.as_deref().is_some_and(|hash| {
            archive
                .as_deref()
                .is_some_and(|archive_path| file_matches(archive_path, hash))
        });

        if state == "ARCHIVING" {
            if !original.exists() && archive_valid {
                connection.execute(
                    "UPDATE downloads SET local_state = 'ARCHIVED', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                    [download_id],
                )?;
                connection.execute(
                    "UPDATE lifecycle_entries SET state = 'ARCHIVED', archived_at = COALESCE(archived_at, CURRENT_TIMESTAMP), last_error = NULL, updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
                    [download_id],
                )?;
            } else if original_valid {
                if let Some(archive_path) = archive.as_deref() {
                    let _ = fs::remove_file(archive_path);
                }
                connection.execute(
                    "UPDATE downloads SET local_state = 'PRESENT', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                    [download_id],
                )?;
                connection.execute(
                    "UPDATE lifecycle_entries SET state = 'ACTIVE', last_error = 'Recovered interrupted archive before original deletion', updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
                    [download_id],
                )?;
            } else {
                connection.execute(
                    "UPDATE downloads SET local_state = 'LOCAL_MISSING', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                    [download_id],
                )?;
                connection.execute(
                    "UPDATE lifecycle_entries SET state = 'ERROR', last_error = 'Could not reconcile interrupted archive safely', updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
                    [download_id],
                )?;
            }
        } else if original_valid {
            if let Some(archive_path) = archive.as_deref() {
                let _ = fs::remove_file(archive_path);
            }
            connection.execute(
                "UPDATE downloads SET local_state = 'PRESENT', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                [download_id],
            )?;
            connection.execute(
                "UPDATE lifecycle_entries SET state = 'ACTIVE', restored_at = COALESCE(restored_at, CURRENT_TIMESTAMP), last_error = NULL, updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
                [download_id],
            )?;
        } else if archive_valid {
            connection.execute(
                "UPDATE downloads SET local_state = 'ARCHIVED', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                [download_id],
            )?;
            connection.execute(
                "UPDATE lifecycle_entries SET state = 'ARCHIVED', last_error = 'Recovered interrupted restore; archive copy remains intact', updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
                [download_id],
            )?;
        } else {
            connection.execute(
                "UPDATE downloads SET local_state = 'LOCAL_MISSING', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                [download_id],
            )?;
            connection.execute(
                "UPDATE lifecycle_entries SET state = 'ERROR', last_error = 'Could not reconcile interrupted restore safely', updated_at = CURRENT_TIMESTAMP WHERE download_id = ?1",
                [download_id],
            )?;
        }
    }
    Ok(())
}

fn file_matches(path: &Path, expected_hash: &str) -> bool {
    path.is_file()
        && crate::storage::sha256_file(path)
            .map(|hash| hash == expected_hash)
            .unwrap_or(false)
}

fn database_health(connection: &Connection) -> rusqlite::Result<String> {
    let quick_check: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if quick_check != "ok" {
        return Ok(format!("QUICK_CHECK_FAILED: {quick_check}"));
    }
    let foreign_key_violation: Option<String> = connection
        .query_row(
            "SELECT printf('%s:%s', \"table\", rowid) FROM pragma_foreign_key_check LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(violation) = foreign_key_violation {
        Ok(format!("FOREIGN_KEY_CHECK_FAILED: {violation}"))
    } else {
        Ok("OK".into())
    }
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
        std::env::temp_dir().join(format!("originkeep-phase4-{unique}-{name}"))
    }

    fn seed_download(
        connection: &Connection,
        id: i64,
        local_path: &Path,
        hash: &str,
        status: &str,
        version: i64,
        duplicate_of_id: Option<i64>,
    ) {
        connection
            .execute(
                r#"
                INSERT INTO downloads (
                    id, capture_key, browser_download_id, original_url, local_path, file_name,
                    sha256, status, browser_state, source_identity, version_number,
                    duplicate_of_id, local_state
                ) VALUES (?1, ?2, ?3, 'https://example.com/report.pdf', ?4, ?5,
                          ?6, ?7, 'complete', 'https://example.com/report.pdf', ?8, ?9, 'PRESENT')
                "#,
                params![
                    id,
                    format!("capture-{id}"),
                    id,
                    local_path.display().to_string(),
                    local_path.file_name().unwrap().to_string_lossy(),
                    hash,
                    status,
                    version,
                    duplicate_of_id,
                ],
            )
            .unwrap();
    }

    #[test]
    fn retention_policy_never_selects_latest_primary_version() {
        let download = StoredDownload {
            id: 1,
            file_name: "report.pdf".into(),
            local_path: unique_path("report.pdf").display().to_string(),
            bytes: Some(10),
            status: "SUPERSEDED".into(),
            local_state: "PRESENT".into(),
            source_identity: Some("https://example.com/report.pdf".into()),
            version_number: Some(2),
            duplicate_of_id: None,
            sha256: Some("abc".into()),
            max_family_version: Some(3),
            lifecycle_state: "ACTIVE".into(),
            archive_path: None,
        };
        assert!(outside_retention_window(&download, 1));
        assert!(!outside_retention_window(&download, 2));
    }

    #[test]
    fn archive_and_restore_round_trip_preserves_bytes() {
        let database_path = unique_path("roundtrip.db");
        crate::phase3::initialize_database(&database_path).unwrap();
        let connection = Connection::open(&database_path).unwrap();
        initialize_connection(&connection).unwrap();
        let original_path = unique_path("report.txt");
        fs::write(&original_path, b"version one").unwrap();
        let hash = crate::storage::sha256_file(&original_path).unwrap();
        seed_download(&connection, 1, &original_path, &hash, "SUPERSEDED", 1, None);
        let latest_path = unique_path("report-latest.txt");
        fs::write(&latest_path, b"version two").unwrap();
        let latest_hash = crate::storage::sha256_file(&latest_path).unwrap();
        seed_download(
            &connection,
            2,
            &latest_path,
            &latest_hash,
            "SOURCE_UNKNOWN",
            2,
            None,
        );
        drop(connection);

        let archived = archive_download(&database_path, 1).unwrap();
        assert_eq!(archived.lifecycle_state, "ARCHIVED");
        assert!(!original_path.exists());
        assert!(archived.restore_eligible);

        let restored = restore_download(&database_path, 1).unwrap();
        assert_eq!(restored.lifecycle_state, "ACTIVE");
        assert!(original_path.is_file());
        assert_eq!(crate::storage::sha256_file(&original_path).unwrap(), hash);

        fs::remove_file(original_path).ok();
        fs::remove_file(latest_path).ok();
        fs::remove_file(database_path).ok();
    }

    #[test]
    fn restore_refuses_to_overwrite_different_bytes() {
        let database_path = unique_path("collision.db");
        crate::phase3::initialize_database(&database_path).unwrap();
        let connection = Connection::open(&database_path).unwrap();
        initialize_connection(&connection).unwrap();
        let original_path = unique_path("collision.txt");
        fs::write(&original_path, b"tracked bytes").unwrap();
        let hash = crate::storage::sha256_file(&original_path).unwrap();
        seed_download(
            &connection,
            1,
            &original_path,
            &hash,
            "DUPLICATE",
            1,
            Some(99),
        );
        drop(connection);

        archive_download(&database_path, 1).unwrap();
        fs::write(&original_path, b"new unrelated bytes").unwrap();
        let error = restore_download(&database_path, 1).unwrap_err();
        assert!(error.contains("refused to overwrite"));

        fs::remove_file(original_path).ok();
        fs::remove_file(database_path).ok();
    }

    #[test]
    fn migration_advances_database_to_phase4() {
        let database_path = unique_path("migration.db");
        initialize_database(&database_path).unwrap();
        let connection = Connection::open(&database_path).unwrap();
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert!(
            version >= 4,
            "expected Phase 4 or newer schema, got {version}"
        );
        assert_eq!(database_health(&connection).unwrap(), "OK");
        drop(connection);
        fs::remove_file(database_path).ok();
    }
}
