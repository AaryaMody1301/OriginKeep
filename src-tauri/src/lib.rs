mod model;
pub mod native_host;
mod storage;

use model::{DownloadRecord, VerificationSummary};
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = storage::default_database_path()
        .and_then(|path| {
            storage::initialize_database(&path)?;
            Ok(path)
        })
        .expect("OriginKeep could not initialize its local database");

    tauri::Builder::default()
        .manage(AppState { database_path })
        .invoke_handler(tauri::generate_handler![list_downloads, verify_local_files])
        .run(tauri::generate_context!())
        .expect("error while running OriginKeep");
}
