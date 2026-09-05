use serde::Serialize;
use serde_json::json;
use std::{env, fs, path::{Path, PathBuf}};

const HOST_NAME: &str = "com.originkeep.host";
const CHROMIUM_EXTENSION_ID: &str = "mplmkmbnahpggimgfihfgieamonbbobh";
const FIREFOX_EXTENSION_ID: &str = "originkeep@originkeep.app";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSetupResult {
    pub platform: String,
    pub host_path: String,
    pub manifests_written: Vec<String>,
    pub note: String,
}

pub fn install_browser_integration() -> Result<BrowserSetupResult, String> {
    #[cfg(windows)]
    {
        return Ok(BrowserSetupResult {
            platform: "windows".into(),
            host_path: sibling_host()?.display().to_string(),
            manifests_written: Vec::new(),
            note: "The OriginKeep NSIS installer registers Chrome, Edge and Firefox native messaging in HKCU. Reinstall the current OriginKeep package to repair those registrations.".into(),
        });
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        install_unix_browser_integration()
    }

    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        Err("Browser integration is currently implemented for Windows, macOS and Linux".into())
    }
}

fn sibling_host() -> Result<PathBuf, String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let directory = executable
        .parent()
        .ok_or_else(|| "Could not determine the OriginKeep executable directory".to_string())?;
    let candidate = directory.join(if cfg!(windows) {
        "originkeep-native-host.exe"
    } else {
        "originkeep-native-host"
    });
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "Bundled OriginKeep native host was not found at {}",
            candidate.display()
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn install_unix_browser_integration() -> Result<BrowserSetupResult, String> {
    let source = sibling_host()?;
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable".to_string())?;
    let data_root = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/OriginKeep")
    } else if let Some(value) = env::var_os("XDG_DATA_HOME") {
        PathBuf::from(value).join("OriginKeep")
    } else {
        home.join(".local/share/OriginKeep")
    };
    let bin_dir = data_root.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|error| error.to_string())?;
    let installed_host = bin_dir.join("originkeep-native-host");
    fs::copy(&source, &installed_host).map_err(|error| error.to_string())?;
    make_executable(&installed_host)?;

    let chromium = json!({
        "name": HOST_NAME,
        "description": "OriginKeep local provenance capture host",
        "path": installed_host.display().to_string(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{CHROMIUM_EXTENSION_ID}/")]
    });
    let firefox = json!({
        "name": HOST_NAME,
        "description": "OriginKeep local provenance capture host",
        "path": installed_host.display().to_string(),
        "type": "stdio",
        "allowed_extensions": [FIREFOX_EXTENSION_ID]
    });

    let mut manifests_written = Vec::new();
    for directory in chromium_manifest_directories(&home) {
        manifests_written.push(write_manifest(&directory, &chromium)?);
    }
    manifests_written.push(write_manifest(&firefox_manifest_directory(&home), &firefox)?);

    Ok(BrowserSetupResult {
        platform: env::consts::OS.into(),
        host_path: installed_host.display().to_string(),
        manifests_written,
        note: "Native messaging was installed per-user. Basic provenance works without page host permissions; enhanced page context remains opt-in in the browser companion.".into(),
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn write_manifest(directory: &Path, value: &serde_json::Value) -> Result<String, String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("{HOST_NAME}.json"));
    let contents = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&path, format!("{contents}\n")).map_err(|error| error.to_string())?;
    Ok(path.display().to_string())
}

#[cfg(target_os = "macos")]
fn chromium_manifest_directories(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts"),
        home.join("Library/Application Support/Chromium/NativeMessagingHosts"),
        home.join("Library/Application Support/Microsoft Edge/NativeMessagingHosts"),
        home.join("Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        home.join("Library/Application Support/Vivaldi/NativeMessagingHosts"),
    ]
}

#[cfg(target_os = "linux")]
fn chromium_manifest_directories(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".config/google-chrome/NativeMessagingHosts"),
        home.join(".config/chromium/NativeMessagingHosts"),
        home.join(".config/microsoft-edge/NativeMessagingHosts"),
        home.join(".config/BraveSoftware/Brave-Browser/NativeMessagingHosts"),
        home.join(".config/vivaldi/NativeMessagingHosts"),
    ]
}

#[cfg(target_os = "macos")]
fn firefox_manifest_directory(home: &Path) -> PathBuf {
    home.join("Library/Application Support/Mozilla/NativeMessagingHosts")
}

#[cfg(target_os = "linux")]
fn firefox_manifest_directory(home: &Path) -> PathBuf {
    home.join(".mozilla/native-messaging-hosts")
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|error| error.to_string())?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|error| error.to_string())
}
