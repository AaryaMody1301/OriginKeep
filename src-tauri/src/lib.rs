mod model;
pub mod native_host;
mod phase3;
mod phase4;
mod storage;

use model::{DownloadRecord, VerificationSummary};
use phase3::{ComparisonResult, RemoteEvidence};
use phase4::{LifecycleItem, LifecycleReview};
use std::path::PathBuf;

struct AppState {
    database_path: PathBuf,
}

#[tauri::command]
fn list_downloads(
    state: tauri::State<'_, AppState>,
    query: Option<String>,
) -> Result<Vec<DownloadRecord>, String> {
    storage::list_downloads(&state.database_path, query.as_deref())
}

#[tauri::command]
fn verify_local_files(state: tauri::State<'_, AppState>) -> Result<VerificationSummary, String> {
    storage::verify_local_files(&state.database_path)
}

#[tauri::command]
fn list_remote_evidence(state: tauri::State<'_, AppState>) -> Result<Vec<RemoteEvidence>, String> {
    phase3::list_remote_evidence(&state.database_path)
}

#[tauri::command]
fn check_remote_freshness(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<RemoteEvidence, String> {
    phase3::check_remote_freshness(&state.database_path, download_id)
}

#[tauri::command]
fn compare_with_previous(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<ComparisonResult, String> {
    phase3::compare_with_previous(&state.database_path, download_id)
}

#[tauri::command]
fn lifecycle_review(
    state: tauri::State<'_, AppState>,
    keep_latest_versions: i64,
    include_duplicates: bool,
) -> Result<LifecycleReview, String> {
    phase4::lifecycle_review(
        &state.database_path,
        keep_latest_versions,
        include_duplicates,
    )
}

#[tauri::command]
fn archive_download(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<LifecycleItem, String> {
    phase4::archive_download(&state.database_path, download_id)
}

#[tauri::command]
fn restore_download(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<LifecycleItem, String> {
    phase4::restore_download(&state.database_path, download_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = storage::default_database_path()
        .and_then(|path| {
            phase4::initialize_database(&path)?;
            Ok(path)
        })
        .expect("OriginKeep could not initialize its local database");

    tauri::Builder::default()
        .manage(AppState { database_path })
        .invoke_handler(tauri::generate_handler![
            list_downloads,
            verify_local_files,
            list_remote_evidence,
            check_remote_freshness,
            compare_with_previous,
            lifecycle_review,
            archive_download,
            restore_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running OriginKeep");
}
