use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::{
    fs,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
};

const MAX_COMPARE_BYTES: u64 = 25 * 1024 * 1024;
const PHASE3_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS remote_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    download_id INTEGER NOT NULL,
    checked_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    request_method TEXT NOT NULL,
    request_url TEXT NOT NULL,
    final_url TEXT,
    http_status INTEGER,
    result_state TEXT NOT NULL,
    etag TEXT,
    last_modified TEXT,
    content_length INTEGER,
    evidence TEXT NOT NULL,
    error TEXT,
    FOREIGN KEY(download_id) REFERENCES downloads(id)
);
CREATE INDEX IF NOT EXISTS idx_remote_checks_download_id ON remote_checks(download_id, id DESC);
"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteEvidence {
    pub download_id: i64,
    pub checked_at: String,
    pub request_method: String,
    pub request_url: String,
    pub final_url: Option<String>,
    pub http_status: Option<i64>,
    pub result_state: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_length: Option<i64>,
    pub evidence: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonResult {
    pub current_id: i64,
    pub previous_id: i64,
    pub kind: String,
    pub current_name: String,
    pub previous_name: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Debug)]
struct CompareFile {
    id: i64,
    path: PathBuf,
    name: String,
    mime_type: Option<String>,
}

#[derive(Debug)]
struct ComparisonRow {
    source_identity: Option<String>,
    version_number: Option<i64>,
    duplicate_of_id: Option<i64>,
    local_path: String,
    file_name: String,
    mime_type: Option<String>,
}

pub fn initialize_database(path: &Path) -> Result<(), String> {
    crate::storage::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())
}

fn initialize_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(PHASE3_SCHEMA)?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 3 {
        connection.execute_batch("PRAGMA user_version = 3;")?;
    }
    Ok(())
}

pub fn list_remote_evidence(path: &Path) -> Result<Vec<RemoteEvidence>, String> {
    initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT rc.download_id, rc.checked_at, rc.request_method, rc.request_url,
                   rc.final_url, rc.http_status, rc.result_state, rc.etag,
                   rc.last_modified, rc.content_length, rc.evidence, rc.error
            FROM remote_checks rc
            JOIN (
                SELECT download_id, MAX(id) AS id
                FROM remote_checks
                GROUP BY download_id
            ) latest ON latest.id = rc.id
            ORDER BY rc.checked_at DESC, rc.id DESC
            "#,
        )
        .map_err(|error| error.to_string())?;

    let rows = statement
        .query_map([], |row| {
            Ok(RemoteEvidence {
                download_id: row.get(0)?,
                checked_at: row.get(1)?,
                request_method: row.get(2)?,
                request_url: row.get(3)?,
                final_url: row.get(4)?,
                http_status: row.get(5)?,
                result_state: row.get(6)?,
                etag: row.get(7)?,
                last_modified: row.get(8)?,
                content_length: row.get(9)?,
                evidence: row.get(10)?,
                error: row.get(11)?,
            })
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

pub fn compare_with_previous(path: &Path, download_id: i64) -> Result<ComparisonResult, String> {
    initialize_database(path)?;
    let (current, previous) = load_comparison_pair(path, download_id)?;
    ensure_comparable_size(&current.path)?;
    ensure_comparable_size(&previous.path)?;

    let kind = comparison_kind(&current.path, current.mime_type.as_deref())?;
    let previous_kind = comparison_kind(&previous.path, previous.mime_type.as_deref())?;
    if kind != previous_kind {
        return Err(format!(
            "The two versions resolve to different comparison types ({kind} vs {previous_kind})"
        ));
    }

    let (summary, details) = match kind.as_str() {
        "CSV" => compare_csv_files(&previous.path, &current.path)?,
        "PDF text" => {
            let previous_text = extract_pdf_text(&previous.path)?;
            let current_text = extract_pdf_text(&current.path)?;
            compare_text_content(&previous_text, &current_text, "PDF text")
        }
        "Text" => {
            let previous_text = fs::read_to_string(&previous.path)
                .map_err(|error| format!("Could not read {}: {error}", previous.name))?;
            let current_text = fs::read_to_string(&current.path)
                .map_err(|error| format!("Could not read {}: {error}", current.name))?;
            compare_text_content(&previous_text, &current_text, "Text")
        }
        _ => unreachable!(),
    };

    Ok(ComparisonResult {
        current_id: current.id,
        previous_id: previous.id,
        kind,
        current_name: current.name,
        previous_name: previous.name,
        summary,
        details,
    })
}

fn load_comparison_pair(
    path: &Path,
    download_id: i64,
) -> Result<(CompareFile, CompareFile), String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    initialize_connection(&connection).map_err(|error| error.to_string())?;
    let current: Option<ComparisonRow> = connection
        .query_row(
            r#"
            SELECT source_identity, version_number, duplicate_of_id, local_path, file_name, mime_type
            FROM downloads WHERE id = ?1
            "#,
            [download_id],
            |row| {
                Ok(ComparisonRow {
                    source_identity: row.get(0)?,
                    version_number: row.get(1)?,
                    duplicate_of_id: row.get(2)?,
                    local_path: row.get(3)?,
                    file_name: row.get(4)?,
                    mime_type: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let ComparisonRow {
        source_identity,
        version_number,
        duplicate_of_id,
        local_path: current_path,
        file_name: current_name,
        mime_type: current_mime,
    } = current.ok_or_else(|| format!("Download record #{download_id} does not exist"))?;
    if duplicate_of_id.is_some() {
        return Err("Compare the primary version rather than an exact duplicate".into());
    }
    let source_identity = source_identity
        .ok_or_else(|| "This download has no deterministic source family".to_string())?;
    let version_number = version_number
        .ok_or_else(|| "This download has no deterministic version number".to_string())?;
    if version_number <= 1 {
        return Err("This source family has no previous version to compare".into());
    }

    let previous_version = version_number - 1;
    let previous: Option<(i64, String, String, Option<String>)> = connection
        .query_row(
            r#"
            SELECT id, local_path, file_name, mime_type
            FROM downloads
            WHERE source_identity = ?1
              AND version_number = ?2
              AND duplicate_of_id IS NULL
            ORDER BY id ASC
            LIMIT 1
            "#,
            params![source_identity, previous_version],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (previous_id, previous_path, previous_name, previous_mime) = previous
        .ok_or_else(|| format!("Primary version {previous_version} is not available locally"))?;

    let current = CompareFile {
        id: download_id,
        path: PathBuf::from(current_path),
        name: current_name,
        mime_type: current_mime,
    };
    let previous = CompareFile {
        id: previous_id,
        path: PathBuf::from(previous_path),
        name: previous_name,
        mime_type: previous_mime,
    };
    if !current.path.is_file() {
        return Err(format!(
            "Current file is missing: {}",
            current.path.display()
        ));
    }
    if !previous.path.is_file() {
        return Err(format!(
            "Previous file is missing: {}",
            previous.path.display()
        ));
    }
    Ok((current, previous))
}

fn ensure_comparable_size(path: &Path) -> Result<(), String> {
    let bytes = fs::metadata(path)
        .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?
        .len();
    if bytes > MAX_COMPARE_BYTES {
        return Err(format!(
            "Local comparison is capped at 25 MiB per file in Phase 3; {} is {:.1} MiB",
            path.display(),
            bytes as f64 / 1024.0 / 1024.0
        ));
    }
    Ok(())
}

fn comparison_kind(path: &Path, mime_type: Option<&str>) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if mime_type == Some("application/pdf") || extension == "pdf" {
        return Ok("PDF text".into());
    }
    if mime_type == Some("text/csv") || extension == "csv" {
        return Ok("CSV".into());
    }
    let text_extension = matches!(
        extension.as_str(),
        "txt"
            | "md"
            | "json"
            | "log"
            | "xml"
            | "html"
            | "css"
            | "js"
            | "jsx"
            | "ts"
            | "tsx"
            | "py"
            | "sql"
            | "yaml"
            | "yml"
            | "toml"
    );
    if mime_type.is_some_and(|value| value.starts_with("text/")) || text_extension {
        return Ok("Text".into());
    }
    Err("Phase 3 comparison supports local PDF text layers, CSV, and UTF-8 text files".into())
}

fn extract_pdf_text(path: &Path) -> Result<String, String> {
    let owned = path.to_path_buf();
    match catch_unwind(AssertUnwindSafe(|| pdf_extract::extract_text(&owned))) {
        Ok(Ok(text)) => Ok(text),
        Ok(Err(error)) => Err(format!(
            "Could not extract PDF text from {}: {error}",
            path.display()
        )),
        Err(_) => Err(format!(
            "PDF text extraction failed safely for {}; the file may use an unsupported or malformed text layer",
            path.display()
        )),
    }
}

fn compare_text_content(previous: &str, current: &str, label: &str) -> (String, Vec<String>) {
    let diff = TextDiff::from_lines(previous, current);
    let mut additions = 0usize;
    let mut removals = 0usize;
    let mut details = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => {
                additions += 1;
                if details.len() < 12 {
                    details.push(format!("+ {}", preview_line(change.value())));
                }
            }
            ChangeTag::Delete => {
                removals += 1;
                if details.len() < 12 {
                    details.push(format!("- {}", preview_line(change.value())));
                }
            }
            ChangeTag::Equal => {}
        }
    }

    if additions == 0 && removals == 0 {
        (
            format!("No {label} differences were detected."),
            vec!["The extracted/decoded content is identical.".into()],
        )
    } else {
        (
            format!("{label} comparison: {additions} added line(s), {removals} removed line(s)."),
            details,
        )
    }
}

fn preview_line(value: &str) -> String {
    let cleaned = value.trim_end();
    let mut preview: String = cleaned.chars().take(180).collect();
    if cleaned.chars().count() > 180 {
        preview.push('…');
    }
    if preview.is_empty() {
        "<blank line>".into()
    } else {
        preview
    }
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)
        .map_err(|error| format!("Could not open CSV {}: {error}", path.display()))?;
    let headers = reader
        .headers()
        .map_err(|error| {
            format!(
                "Could not read CSV headers from {}: {error}",
                path.display()
            )
        })?
        .iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let rows = reader
        .records()
        .map(|record| {
            record
                .map(|record| record.iter().map(ToOwned::to_owned).collect::<Vec<_>>())
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((headers, rows))
}

fn compare_csv_files(previous: &Path, current: &Path) -> Result<(String, Vec<String>), String> {
    let (previous_headers, previous_rows) = read_csv(previous)?;
    let (current_headers, current_rows) = read_csv(current)?;
    let headers_changed = previous_headers != current_headers;
    let max_rows = previous_rows.len().max(current_rows.len());
    let max_columns = previous_headers.len().max(current_headers.len());
    let mut changed_cells = 0usize;
    let mut details = Vec::new();

    if headers_changed {
        details.push(format!(
            "Headers changed: [{}] -> [{}]",
            previous_headers.join(", "),
            current_headers.join(", ")
        ));
    }

    for row_index in 0..max_rows {
        for column_index in 0..max_columns {
            let before = previous_rows
                .get(row_index)
                .and_then(|row| row.get(column_index));
            let after = current_rows
                .get(row_index)
                .and_then(|row| row.get(column_index));
            if before != after {
                changed_cells += 1;
                if details.len() < 12 {
                    let column = current_headers
                        .get(column_index)
                        .or_else(|| previous_headers.get(column_index))
                        .cloned()
                        .unwrap_or_else(|| format!("column {}", column_index + 1));
                    details.push(format!(
                        "row {}, {}: {:?} -> {:?}",
                        row_index + 2,
                        column,
                        before,
                        after
                    ));
                }
            }
        }
    }

    let summary = format!(
        "CSV comparison: {} -> {} data row(s), {} -> {} column(s), {} changed cell(s){}.",
        previous_rows.len(),
        current_rows.len(),
        previous_headers.len(),
        current_headers.len(),
        changed_cells,
        if headers_changed {
            ", headers changed"
        } else {
            ""
        }
    );
    if details.is_empty() {
        details.push("The parsed CSV content is identical.".into());
    }
    Ok((summary, details))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_comparison_reports_line_changes() {
        let (summary, details) = compare_text_content("alpha\nbeta\n", "alpha\ngamma\n", "Text");
        assert!(summary.contains("1 added line"));
        assert!(summary.contains("1 removed line"));
        assert!(details.iter().any(|detail| detail.contains("beta")));
        assert!(details.iter().any(|detail| detail.contains("gamma")));
    }

    #[test]
    fn comparison_type_is_explicit() {
        assert_eq!(
            comparison_kind(Path::new("report.pdf"), None).unwrap(),
            "PDF text"
        );
        assert_eq!(comparison_kind(Path::new("data.csv"), None).unwrap(), "CSV");
        assert_eq!(
            comparison_kind(Path::new("notes.md"), None).unwrap(),
            "Text"
        );
        assert!(comparison_kind(Path::new("archive.zip"), None).is_err());
    }
}
