use crate::phase4::LifecycleReview;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub fn apply(path: &Path, review: &mut LifecycleReview) -> Result<(), String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    for item in &mut review.items {
        let intent: (String, bool, Option<String>) = connection
            .query_row(
                r#"
                SELECT COALESCE(d.retention_policy, 'MANUAL'),
                       CASE
                         WHEN d.expires_at IS NOT NULL
                          AND datetime(d.expires_at) IS NOT NULL
                          AND datetime(d.expires_at) <= CURRENT_TIMESTAMP
                         THEN 1 ELSE 0
                       END,
                       (
                         SELECT rc.result_state
                         FROM remote_checks rc
                         WHERE rc.download_id = d.id
                         ORDER BY rc.id DESC LIMIT 1
                       )
                FROM downloads d WHERE d.id = ?1
                "#,
                [item.download_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get::<_, i64>(1)? != 0,
                        row.get(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| ("MANUAL".into(), false, None));

        match intent.0.as_str() {
            "NEVER_ARCHIVE" if item.lifecycle_state != "ARCHIVED" => {
                item.reclaimable = false;
                item.recommendation = "PROTECT".into();
                item.reason = "This File Passport explicitly says never archive; OriginKeep will not select it for cleanup.".into();
            }
            "ARCHIVE_WHEN_EXPIRED" if intent.1 && item.archive_eligible => {
                item.reclaimable = true;
                item.recommendation = "ARCHIVE_CANDIDATE".into();
                item.reason = "This File Passport is past its user-defined expiry and the local bytes still satisfy safe-archive requirements.".into();
            }
            "ARCHIVE_WHEN_SUPERSEDED"
                if item.status == "SUPERSEDED" && item.archive_eligible =>
            {
                item.reclaimable = true;
                item.recommendation = "ARCHIVE_CANDIDATE".into();
                item.reason = "This File Passport explicitly allows safe archival after a deterministic newer version supersedes it.".into();
            }
            "REVIEW_WHEN_NEWER" if intent.2.as_deref() == Some("CHANGED") => {
                item.reclaimable = false;
                item.recommendation = "REVIEW_NEWER_SOURCE".into();
                item.reason = "The source has evidence of a newer/changed remote representation. Review it before any lifecycle action.".into();
            }
            _ => {}
        }
    }

    review.summary.reclaimable_bytes = 0;
    review.summary.candidate_count = 0;
    review.summary.protected_bytes = 0;
    for item in &review.items {
        let bytes = item.bytes.unwrap_or(0).max(0);
        if item.reclaimable {
            review.summary.reclaimable_bytes += bytes;
            review.summary.candidate_count += 1;
        } else if item.local_state == "PRESENT" && item.lifecycle_state != "ARCHIVED" {
            review.summary.protected_bytes += bytes;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase4::{LifecycleItem, LifecycleSummary};
    use rusqlite::params;
    use std::{env, fs, time::{SystemTime, UNIX_EPOCH}};

    fn temp_db() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("originkeep-intent-{unique}.db"))
    }

    #[test]
    fn never_archive_overrides_generic_cleanup_candidate() {
        let path = temp_db();
        crate::passport::initialize_database(&path).unwrap();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                r#"
                INSERT INTO downloads (
                    capture_key, browser_download_id, original_url, local_path, file_name,
                    status, browser_state, local_state, retention_policy
                ) VALUES ('intent-test', 1, 'https://example.com/file', '/tmp/file', 'file',
                          'SOURCE_UNKNOWN', 'complete', 'PRESENT', 'NEVER_ARCHIVE')
                "#,
                [],
            )
            .unwrap();
        let id = connection.last_insert_rowid();
        let mut review = LifecycleReview {
            keep_latest_versions: 1,
            include_duplicates: true,
            summary: LifecycleSummary {
                tracked_bytes: 10,
                present_bytes: 10,
                archived_bytes: 0,
                reclaimable_bytes: 10,
                duplicate_bytes: 0,
                superseded_bytes: 0,
                protected_bytes: 0,
                candidate_count: 1,
                archived_count: 0,
                database_health: "ok".into(),
            },
            items: vec![LifecycleItem {
                download_id: id,
                file_name: "file".into(),
                original_path: "/tmp/file".into(),
                bytes: Some(10),
                status: "SOURCE_UNKNOWN".into(),
                local_state: "PRESENT".into(),
                source_identity: None,
                version_number: None,
                duplicate_of_id: None,
                lifecycle_state: "ACTIVE".into(),
                archive_path: None,
                reclaimable: true,
                archive_eligible: true,
                restore_eligible: false,
                recommendation: "ARCHIVE_CANDIDATE".into(),
                reason: "generic".into(),
            }],
        };
        apply(&path, &mut review).unwrap();
        fs::remove_file(path).ok();
        assert!(!review.items[0].reclaimable);
        assert_eq!(review.items[0].recommendation, "PROTECT");
        assert_eq!(review.summary.candidate_count, 0);
    }
}
