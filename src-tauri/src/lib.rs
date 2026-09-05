mod adopt;
mod model;
pub mod native_host;
mod passport;
mod phase3;
mod phase4;
mod secure_remote;
mod storage;
mod trust;

use model::{DownloadRecord, VerificationSummary};
use passport::{FilePassport, MoveScanResult, OriginGraph, PassportExport, RelinkResult};
use phase3::{ComparisonResult, RemoteEvidence};
use phase4::{LifecycleItem, LifecycleReview};
use std::path::PathBuf;
use trust::TrustReport;

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
    secure_remote::check_remote_freshness(&state.database_path, download_id)
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

#[tauri::command]
fn list_passports(state: tauri::State<'_, AppState>) -> Result<Vec<FilePassport>, String> {
    passport::list_passports(&state.database_path)
}

#[tauri::command]
fn update_passport_metadata(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    user_note: Option<String>,
    purpose: Option<String>,
    expires_at: Option<String>,
    retention_action: String,
) -> Result<FilePassport, String> {
    passport::update_metadata(
        &state.database_path,
        download_id,
        user_note,
        purpose,
        expires_at,
        retention_action,
    )
}

#[tauri::command]
fn export_passport(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<PassportExport, String> {
    passport::export_passport(&state.database_path, download_id)
}

#[tauri::command]
fn import_passport(
    state: tauri::State<'_, AppState>,
    passport_path: String,
) -> Result<FilePassport, String> {
    passport::import_passport(&state.database_path, passport_path)
}

#[tauri::command]
fn adopt_file(
    state: tauri::State<'_, AppState>,
    local_path: String,
) -> Result<FilePassport, String> {
    adopt::adopt_file(&state.database_path, local_path)
}

#[tauri::command]
fn relink_file(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    new_path: String,
) -> Result<RelinkResult, String> {
    passport::relink_file(&state.database_path, download_id, new_path)
}

#[tauri::command]
fn scan_for_moves(
    state: tauri::State<'_, AppState>,
    root: String,
    max_files: usize,
) -> Result<MoveScanResult, String> {
    passport::scan_for_moves(&state.database_path, root, max_files)
}

#[tauri::command]
fn import_os_provenance(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<FilePassport, String> {
    passport::import_os_provenance(&state.database_path, download_id)
}

#[tauri::command]
fn origin_graph(state: tauri::State<'_, AppState>) -> Result<OriginGraph, String> {
    passport::origin_graph(&state.database_path)
}

#[tauri::command]
fn inspect_trust(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<TrustReport, String> {
    trust::inspect(&state.database_path, download_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = storage::default_database_path()
        .and_then(|path| {
            passport::initialize_database(&path)?;
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
            restore_download,
            list_passports,
            update_passport_metadata,
            export_passport,
            import_passport,
            adopt_file,
            relink_file,
            scan_for_moves,
            import_os_provenance,
            origin_graph,
            inspect_trust
        ])
        .run(tauri::generate_context!())
        .expect("error while running OriginKeep");
}
