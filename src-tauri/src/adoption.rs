use crate::{model::DownloadCapture, passport, storage};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn adopt_local_file(
    database: &Path,
    file_path: String,
    source_url: Option<String>,
    purpose: Option<String>,
    note: Option<String>,
) -> Result<passport::PassportRecord, String> {
    passport::initialize_database(database)?;
    let file = PathBuf::from(file_path);
    if !file.is_file() {
        return Err(format!("Local file does not exist: {}", file.display()));
    }
    let metadata = fs::metadata(&file).map_err(|error| error.to_string())?;
    let hash = storage::sha256_file(&file)
        .map_err(|error| format!("Could not fingerprint {}: {error}", file.display()))?;
    let original_url = source_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "urn:originkeep:local-adoption".into());
    let capture = DownloadCapture {
        capture_key: format!("desktop-adopt:{hash}:{}", file.display()),
        browser_download_id: 0,
        original_url,
        final_url: None,
        referrer: None,
        local_path: file.display().to_string(),
        file_name: file
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .ok_or_else(|| "Local file has no filename".to_string())?,
        mime_type: None,
        bytes: Some(metadata.len() as i64),
        started_at: None,
        completed_at: None,
        state: "complete".into(),
        page_url: None,
        page_title: None,
        link_text: None,
        context_text: None,
        browser_name: Some("OriginKeep desktop adoption".into()),
    };
    let ingested = storage::ingest_capture(database, &capture)?;
    passport::update_metadata(database, ingested.id, purpose, note, None, "MANUAL".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn adopts_local_bytes_without_inventing_web_origin() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file = env::temp_dir().join(format!("originkeep-adopt-{unique}.txt"));
        let database = env::temp_dir().join(format!("originkeep-adopt-{unique}.db"));
        fs::write(&file, b"existing file").unwrap();
        let record = adopt_local_file(
            &database,
            file.display().to_string(),
            None,
            Some("Reference".into()),
            None,
        )
        .unwrap();
        assert_eq!(record.original_url, "urn:originkeep:local-adoption");
        assert!(record.source_identity.is_none());
        assert_eq!(record.purpose.as_deref(), Some("Reference"));
        fs::remove_file(file).ok();
        fs::remove_file(database).ok();
    }
}
