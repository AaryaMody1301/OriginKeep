use crate::{model::DownloadCapture, passport, storage};
use std::{fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH}};
use url::Url;

pub fn adopt_file(database_path: &Path, local_path: String) -> Result<passport::FilePassport, String> {
    passport::initialize_database(database_path)?;
    let local = PathBuf::from(local_path);
    if !local.is_file() {
        return Err("Adopt requires an existing regular file".into());
    }
    let hash = storage::sha256_file(&local).map_err(|error| error.to_string())?;
    let source = detect_os_source(&local).unwrap_or_else(|| "originkeep://local-adoption".into());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let metadata = fs::metadata(&local).map_err(|error| error.to_string())?;
    let capture = DownloadCapture {
        capture_key: format!("adopt:{hash}:{now}"),
        browser_download_id: 0,
        original_url: source,
        final_url: None,
        referrer: None,
        local_path: local.display().to_string(),
        file_name: local
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .ok_or_else(|| "Adopted path has no file name".to_string())?,
        mime_type: None,
        bytes: Some(metadata.len() as i64),
        started_at: None,
        completed_at: None,
        state: "complete".into(),
        page_title: None,
        page_url: None,
        link_text: None,
        context_text: None,
        browser_name: Some(format!("{} local adoption", std::env::consts::OS)),
    };
    let result = storage::ingest_capture(database_path, &capture)?;
    passport::initialize_database(database_path)?;
    passport::import_os_provenance(database_path, result.id)
}

fn detect_os_source(local: &Path) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let ads = PathBuf::from(format!("{}:Zone.Identifier", local.display()));
        if let Ok(value) = fs::read_to_string(ads) {
            for line in value.lines() {
                if let Some(url) = line.strip_prefix("HostUrl=").or_else(|| line.strip_prefix("ReferrerUrl=")) {
                    if valid_http(url) {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("mdls")
            .args(["-raw", "-name", "kMDItemWhereFroms"])
            .arg(local)
            .output()
        {
            if let Some(url) = first_http_url(&String::from_utf8_lossy(&output.stdout)) {
                return Some(url);
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("gio")
            .args(["info", "--attributes=metadata::download-uri"])
            .arg(local)
            .output()
        {
            if let Some(url) = first_http_url(&String::from_utf8_lossy(&output.stdout)) {
                return Some(url);
            }
        }
    }
    None
}

fn valid_http(value: &str) -> bool {
    Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https"))
        .unwrap_or(false)
}

fn first_http_url(value: &str) -> Option<String> {
    for marker in ["https://", "http://"] {
        if let Some(start) = value.find(marker) {
            let candidate = &value[start..];
            let end = candidate
                .find(|character: char| matches!(character, '"' | '\'' | ')' | ',' | '\n' | '\r' | ' '))
                .unwrap_or(candidate.len());
            let url = &candidate[..end];
            if valid_http(url) {
                return Some(url.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_urls_from_os_metadata_text() {
        assert_eq!(
            first_http_url("kMDItemWhereFroms = (\"https://example.com/report.pdf\", \"https://example.com\")").as_deref(),
            Some("https://example.com/report.pdf")
        );
        assert!(first_http_url("(null)").is_none());
    }
}
