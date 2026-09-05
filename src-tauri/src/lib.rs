mod adopt;
mod browser_setup;
mod model;
pub mod native_host;
mod passport;
mod pending_context;
mod phase3;
mod phase4;
mod secure_remote;
mod storage;

use browser_setup::BrowserSetupResult;
use model::{DownloadRecord, VerificationSummary};
use passport::{
    FilePassport, LocationRefreshSummary, OriginGraph, PassportExportResult, PassportSummary,
    ReconnectResult, TrustObservation,
};
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
fn list_passport_summaries(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<PassportSummary>, String> {
    passport::list_passport_summaries(&state.database_path)
}

#[tauri::command]
fn get_file_passport(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<FilePassport, String> {
    passport::get_file_passport(&state.database_path, download_id)
}

#[tauri::command]
fn update_passport_metadata(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    purpose: String,
    note: Option<String>,
    expires_at: Option<String>,
    sigstore_identity: Option<String>,
    sigstore_issuer: Option<String>,
) -> Result<FilePassport, String> {
    passport::update_passport_metadata(
        &state.database_path,
        download_id,
        purpose,
        note,
        expires_at,
        sigstore_identity,
        sigstore_issuer,
    )
}

#[tauri::command]
fn export_passport(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<PassportExportResult, String> {
    passport::export_passport(&state.database_path, download_id)
}

#[tauri::command]
fn import_passport(
    state: tauri::State<'_, AppState>,
    sidecar_path: String,
) -> Result<FilePassport, String> {
    passport::import_passport(&state.database_path, sidecar_path)
}

#[tauri::command]
fn reconnect_file(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    new_path: String,
) -> Result<ReconnectResult, String> {
    passport::reconnect_file(&state.database_path, download_id, new_path)
}

#[tauri::command]
fn refresh_locations(state: tauri::State<'_, AppState>) -> Result<LocationRefreshSummary, String> {
    passport::refresh_locations(&state.database_path)
}

#[tauri::command]
fn refresh_trust(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<Vec<TrustObservation>, String> {
    passport::refresh_trust(&state.database_path, download_id)
}

#[tauri::command]
fn origin_graph(state: tauri::State<'_, AppState>) -> Result<OriginGraph, String> {
    passport::origin_graph(&state.database_path)
}

#[tauri::command]
fn install_browser_integration() -> Result<BrowserSetupResult, String> {
    browser_setup::install_browser_integration()
}

#[tauri::command]
fn adopt_existing_file(
    state: tauri::State<'_, AppState>,
    file_path: String,
    source_url: Option<String>,
) -> Result<FilePassport, String> {
    adopt::adopt_existing_file(&state.database_path, file_path, source_url)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = storage::default_database_path()
        .and_then(|path| {
            pending_context::initialize_database(&path)?;
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
            list_passport_summaries,
            get_file_passport,
            update_passport_metadata,
            export_passport,
            import_passport,
            reconnect_file,
            refresh_locations,
            refresh_trust,
            origin_graph,
            install_browser_integration,
            adopt_existing_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running OriginKeep");
}
