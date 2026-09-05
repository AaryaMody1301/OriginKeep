use serde::Serialize;
use serde_json::json;
use std::{env, fs, path::{Path, PathBuf}};

pub const CHROMIUM_EXTENSION_ID: &str = "mplmkmbnahpggimgfihfgieamonbbobh";
pub const FIREFOX_EXTENSION_ID: &str = "originkeep@aaryamody1301.github.io";
const HOST_NAME: &str = "com.originkeep.host";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeTarget {
    pub browser: String,
    pub manifest_path: Option<String>,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub platform: String,
    pub native_host_path: Option<String>,
    pub targets: Vec<BridgeTarget>,
    pub safari_note: String,
}

pub fn ensure_registration() -> BridgeStatus {
    if cfg!(target_os = "windows") {
        return BridgeStatus {
            platform: "windows".into(),
            native_host_path: locate_native_host().map(|path| path.display().to_string()),
            targets: vec![
                installer_managed("Chrome"),
                installer_managed("Edge"),
                installer_managed("Firefox"),
            ],
            safari_note: "Safari is not available on Windows.".into(),
        };
    }

    let Some(host) = locate_native_host() else {
        return BridgeStatus {
            platform: env::consts::OS.into(),
            native_host_path: None,
            targets: Vec::new(),
            safari_note: safari_note(),
        };
    };
    let host = fs::canonicalize(&host).unwrap_or(host);
    let mut targets = Vec::new();
    for target in browser_manifest_targets() {
        let manifest = if target.firefox {
            firefox_manifest(&host)
        } else {
            chromium_manifest(&host)
        };
        let result = write_manifest(&target.directory, &manifest);
        targets.push(BridgeTarget {
            browser: target.browser.into(),
            manifest_path: result.as_ref().ok().map(|path| path.display().to_string()),
            state: if result.is_ok() { "REGISTERED" } else { "UNAVAILABLE" }.into(),
            detail: match result {
                Ok(path) => format!("Native Messaging manifest available at {}", path.display()),
                Err(error) => error,
            },
        });
    }
    BridgeStatus {
        platform: env::consts::OS.into(),
        native_host_path: Some(host.display().to_string()),
        targets,
        safari_note: safari_note(),
    }
}

fn installer_managed(browser: &str) -> BridgeTarget {
    BridgeTarget {
        browser: browser.into(),
        manifest_path: None,
        state: "INSTALLER_MANAGED".into(),
        detail: "The Windows NSIS installer owns this per-user Native Messaging registration and removes it on uninstall.".into(),
    }
}

struct BrowserManifestTarget {
    browser: &'static str,
    directory: PathBuf,
    firefox: bool,
}

#[cfg(target_os = "macos")]
fn browser_manifest_targets() -> Vec<BrowserManifestTarget> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    vec![
        BrowserManifestTarget {
            browser: "Chrome",
            directory: home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts"),
            firefox: false,
        },
        BrowserManifestTarget {
            browser: "Chromium",
            directory: home.join("Library/Application Support/Chromium/NativeMessagingHosts"),
            firefox: false,
        },
        BrowserManifestTarget {
            browser: "Edge",
            directory: home.join("Library/Application Support/Microsoft Edge/NativeMessagingHosts"),
            firefox: false,
        },
        BrowserManifestTarget {
            browser: "Firefox",
            directory: home.join("Library/Application Support/Mozilla/NativeMessagingHosts"),
            firefox: true,
        },
    ]
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn browser_manifest_targets() -> Vec<BrowserManifestTarget> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    vec![
        BrowserManifestTarget {
            browser: "Chrome",
            directory: home.join(".config/google-chrome/NativeMessagingHosts"),
            firefox: false,
        },
        BrowserManifestTarget {
            browser: "Chromium",
            directory: home.join(".config/chromium/NativeMessagingHosts"),
            firefox: false,
        },
        BrowserManifestTarget {
            browser: "Edge",
            directory: home.join(".config/microsoft-edge/NativeMessagingHosts"),
            firefox: false,
        },
        BrowserManifestTarget {
            browser: "Firefox",
            directory: home.join(".mozilla/native-messaging-hosts"),
            firefox: true,
        },
    ]
}

#[cfg(target_os = "windows")]
fn browser_manifest_targets() -> Vec<BrowserManifestTarget> {
    Vec::new()
}

fn locate_native_host() -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let parent = executable.parent()?;
    let extension = if cfg!(target_os = "windows") { ".exe" } else { "" };
    let name = format!("originkeep-native-host{extension}");
    let candidates = [
        parent.join(&name),
        parent.join(".." ).join("Resources").join(&name),
        parent.join(".." ).join("MacOS").join(&name),
    ];
    candidates.into_iter().find(|path| path.is_file())
}

fn chromium_manifest(host: &Path) -> serde_json::Value {
    json!({
        "name": HOST_NAME,
        "description": "OriginKeep local provenance capture host",
        "path": host.display().to_string(),
        "type": "stdio",
        "allowed_origins": [format!("chrome-extension://{CHROMIUM_EXTENSION_ID}/")]
    })
}

fn firefox_manifest(host: &Path) -> serde_json::Value {
    json!({
        "name": HOST_NAME,
        "description": "OriginKeep local provenance capture host",
        "path": host.display().to_string(),
        "type": "stdio",
        "allowed_extensions": [FIREFOX_EXTENSION_ID]
    })
}

fn write_manifest(directory: &Path, manifest: &serde_json::Value) -> Result<PathBuf, String> {
    fs::create_dir_all(directory)
        .map_err(|error| format!("Could not create {}: {error}", directory.display()))?;
    let target = directory.join(format!("{HOST_NAME}.json"));
    let payload = serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?;
    fs::write(&target, payload)
        .map_err(|error| format!("Could not write {}: {error}", target.display()))?;
    Ok(target)
}

fn safari_note() -> String {
    if cfg!(target_os = "macos") {
        "Safari Web Extensions use a containing macOS app/native app-extension bridge. Apple's current Safari packager warns that the downloads manifest capability is unsupported, so OriginKeep does not claim automatic Safari download-event parity. Portable passports and desktop features remain available.".into()
    } else {
        "Safari's browser extension bridge is macOS-specific and is documented separately.".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_keep_browser_specific_allowlists_separate() {
        let host = Path::new("/tmp/originkeep-native-host");
        let chrome = chromium_manifest(host);
        let firefox = firefox_manifest(host);
        assert!(chrome.get("allowed_origins").is_some());
        assert!(chrome.get("allowed_extensions").is_none());
        assert!(firefox.get("allowed_extensions").is_some());
        assert!(firefox.get("allowed_origins").is_none());
    }
}
