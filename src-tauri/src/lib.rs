mod adoption;
mod intent_policy;
mod model;
pub mod native_host;
mod passport;
mod phase3;
mod phase4;
mod platform_bridge;
mod secure_remote;
mod storage;
mod trust;

use model::{DownloadRecord, VerificationSummary};
use passport::{OriginGraph, PassportExport, PassportRecord, RelinkResult};
use phase3::{ComparisonResult, RemoteEvidence};
use phase4::{LifecycleItem, LifecycleReview};
use platform_bridge::BridgeStatus;
use std::path::PathBuf;
use trust::{SigstoreVerification, TrustLens};

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
    let mut review = phase4::lifecycle_review(
        &state.database_path,
        keep_latest_versions,
        include_duplicates,
    )?;
    intent_policy::apply(&state.database_path, &mut review)?;
    Ok(review)
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
fn list_passports(state: tauri::State<'_, AppState>) -> Result<Vec<PassportRecord>, String> {
    passport::list_passports(&state.database_path)
}

#[tauri::command]
fn get_passport(
    state: tauri::State<'_, AppState>,
    download_id: i64,
) -> Result<PassportRecord, String> {
    passport::get_passport(&state.database_path, download_id)
}

#[tauri::command]
fn update_passport_metadata(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    purpose: Option<String>,
    note: Option<String>,
    expires_at: Option<String>,
    retention_policy: String,
) -> Result<PassportRecord, String> {
    passport::update_metadata(
        &state.database_path,
        download_id,
        purpose,
        note,
        expires_at,
        retention_policy,
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
    file_path: String,
) -> Result<PassportRecord, String> {
    passport::import_passport(&state.database_path, passport_path, file_path)
}

#[tauri::command]
fn adopt_local_file(
    state: tauri::State<'_, AppState>,
    file_path: String,
    source_url: Option<String>,
    purpose: Option<String>,
    note: Option<String>,
) -> Result<PassportRecord, String> {
    adoption::adopt_local_file(&state.database_path, file_path, source_url, purpose, note)
}

#[tauri::command]
fn relink_download(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    candidate_path: String,
) -> Result<PassportRecord, String> {
    passport::relink_download(&state.database_path, download_id, candidate_path)
}

#[tauri::command]
fn find_moved_file(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    search_root: String,
) -> Result<RelinkResult, String> {
    passport::find_moved_file(&state.database_path, download_id, search_root)
}

#[tauri::command]
fn origin_graph(state: tauri::State<'_, AppState>) -> Result<OriginGraph, String> {
    passport::origin_graph(&state.database_path)
}

#[tauri::command]
fn inspect_trust(state: tauri::State<'_, AppState>, download_id: i64) -> Result<TrustLens, String> {
    trust::inspect(&state.database_path, download_id)
}

#[tauri::command]
fn verify_sigstore(
    state: tauri::State<'_, AppState>,
    download_id: i64,
    identity: String,
    issuer: String,
) -> Result<SigstoreVerification, String> {
    trust::verify_sigstore(&state.database_path, download_id, identity, issuer)
}

#[tauri::command]
fn browser_bridge_status() -> BridgeStatus {
    platform_bridge::ensure_registration()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = storage::default_database_path()
        .and_then(|path| {
            passport::initialize_database(&path)?;
            Ok(path)
        })
        .expect("OriginKeep could not initialize its local database");

    let _ = platform_bridge::ensure_registration();

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
            get_passport,
            update_passport_metadata,
            export_passport,
            import_passport,
            adopt_local_file,
            relink_download,
            find_moved_file,
            origin_graph,
            inspect_trust,
            verify_sigstore,
            browser_bridge_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running OriginKeep");
}
