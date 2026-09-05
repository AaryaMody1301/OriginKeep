use crate::{
    model::DownloadCapture,
    passport::{self, CaptureContext, FilePassport},
    pending_context, storage,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[cfg(target_os = "macos")]
use std::process::Command;

pub fn adopt_existing_file(
    database: &Path,
    file_path: String,
    source_url: Option<String>,
) -> Result<FilePassport, String> {
    passport::initialize_database(database)?;
    let path = PathBuf::from(file_path.trim());
    if !path.is_file() {
        return Err(format!("Selected path is not a file: {}", path.display()));
    }
    let hash = storage::sha256_file(&path).map_err(|error| error.to_string())?;
    let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Selected file has no valid filename".to_string())?
        .to_string();
    let evidence = os_provenance(&path);
    let pending = pending_context::recent(database).unwrap_or(None);
    let explicit_source = source_url
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let original_url = explicit_source
        .clone()
        .or_else(|| evidence.host_url.clone())
        .or_else(|| evidence.where_from.first().cloned())
        .unwrap_or_else(|| "originkeep://local-file".into());
    let referrer = evidence
        .referrer_url
        .clone()
        .or_else(|| evidence.where_from.get(1).cloned());
    let capture = DownloadCapture {
        capture_key: format!("adopt:{hash}:{}", path.display()),
        browser_download_id: 0,
        original_url,
        final_url: explicit_source,
        referrer,
        local_path: path.display().to_string(),
        file_name,
        mime_type: None,
        bytes: i64::try_from(metadata.len()).ok(),
        started_at: None,
        completed_at: None,
        state: "complete".into(),
    };
    let result = storage::ingest_capture(database, &capture)?;

    let pending_context_text = pending.as_ref().and_then(|value| value.context_text.clone());
    let context_text = match (pending_context_text, evidence.summary.clone()) {
        (Some(context), Some(summary)) => Some(format!("{context}\n\n{summary}")),
        (Some(context), None) => Some(context),
        (None, summary) => summary,
    };
    let context = CaptureContext {
        browser_name: pending
            .as_ref()
            .and_then(|value| value.browser_name.clone())
            .or_else(|| Some("OS / existing file".into())),
        page_title: pending.as_ref().and_then(|value| value.page_title.clone()),
        page_url: pending
            .as_ref()
            .and_then(|value| value.page_url.clone())
            .or_else(|| evidence.host_url.clone())
            .or_else(|| evidence.where_from.first().cloned()),
        link_text: pending.as_ref().and_then(|value| value.link_text.clone()),
        context_text,
        context_source: Some(match pending.as_ref() {
            Some(_) => format!("safari-fallback+{}", evidence.source),
            None => evidence.source,
        }),
    };
    passport::record_capture(database, &capture, &result, &context)?;
    passport::get_file_passport(database, result.id)
}

#[derive(Default)]
struct OsProvenance {
    host_url: Option<String>,
    referrer_url: Option<String>,
    where_from: Vec<String>,
    summary: Option<String>,
    source: String,
}

#[cfg(windows)]
fn os_provenance(path: &Path) -> OsProvenance {
    let ads = format!("{}:Zone.Identifier", path.display());
    let Ok(content) = fs::read_to_string(ads) else {
        return OsProvenance {
            source: "existing-file".into(),
            ..Default::default()
        };
    };
    let value = |key: &str| {
        content.lines().find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            candidate
                .trim()
                .eq_ignore_ascii_case(key)
                .then(|| value.trim().to_string())
        })
    };
    OsProvenance {
        host_url: value("HostUrl"),
        referrer_url: value("ReferrerUrl"),
        where_from: Vec::new(),
        summary: value("ZoneId")
            .map(|zone| format!("Imported from Windows Zone.Identifier (ZoneId {zone}).")),
        source: "windows-zone-identifier".into(),
    }
}

#[cfg(target_os = "macos")]
fn os_provenance(path: &Path) -> OsProvenance {
    let output = Command::new("mdls")
        .args(["-raw", "-name", "kMDItemWhereFroms"])
        .arg(path)
        .output();
    let Ok(output) = output else {
        return OsProvenance {
            source: "existing-file".into(),
            ..Default::default()
        };
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let where_from = quoted_strings(&text)
        .into_iter()
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        .collect::<Vec<_>>();
    OsProvenance {
        host_url: where_from.first().cloned(),
        referrer_url: where_from.get(1).cloned(),
        summary: (!where_from.is_empty())
            .then(|| "Imported from macOS kMDItemWhereFroms metadata.".into()),
        where_from,
        source: "macos-where-froms".into(),
    }
}

#[cfg(target_os = "macos")]
fn quoted_strings(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' && in_quote {
            escaped = true;
        } else if character == '"' {
            if in_quote {
                values.push(current.clone());
                current.clear();
            }
            in_quote = !in_quote;
        } else if in_quote {
            current.push(character);
        }
    }
    values
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn os_provenance(_path: &Path) -> OsProvenance {
    OsProvenance {
        source: "existing-file".into(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use super::quoted_strings;

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_macos_where_froms_output_without_guessing() {
        let values = quoted_strings(
            "(\n  \"https://example.com/file\",\n  \"https://example.com/page\"\n)",
        );
        assert_eq!(values.len(), 2);
    }
}
