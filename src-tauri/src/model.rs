use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadCapture {
    pub capture_key: String,
    pub browser_download_id: i64,
    pub original_url: String,
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub local_path: String,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    pub id: i64,
    pub capture_key: String,
    pub original_url: String,
    pub final_url: Option<String>,
    pub referrer: Option<String>,
    pub local_path: String,
    pub file_name: String,
    pub mime_type: Option<String>,
    pub bytes: Option<i64>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub sha256: Option<String>,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResult {
    pub ok: bool,
    pub id: i64,
    pub sha256: Option<String>,
    pub status: String,
}
