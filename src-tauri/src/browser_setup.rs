use serde::Serialize;
use serde_json::json;
use std::{env, fs, path::{Path, PathBuf}};

const HOST_NAME: &str = "com.originkeep.host";
const CHROMIUM_ID: &str = "mplmkmbnahpggimgfihfgieamonbbobh";
const FIREFOX_ID: &str = "originkeep@aaryamody.local";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSetupResult {
    pub platform: String,
    pub native_host_path: String,
    pub manifests: Vec<String>,
    pub note: String,
}

pub fn register() -> Result<BrowserSetupResult, String> {
    #[cfg(target_os = "windows")]
    {
        return Ok(BrowserSetupResult {
            platform: "windows".into(),
            native_host_path: sibling_host()?.display().to_string(),
            manifests: Vec::new(),
            note: "The OriginKeep NSIS installer owns Chrome, Edge and Firefox native-messaging registration on Windows.".into(),
        });
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let host = sibling_host()?;
        if !host.is_file() {
            return Err(format!("Bundled native host was not found at {}", host.display()));
        }
        let home = env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is unavailable; browser integration cannot be installed".to_string())?;
        let chromium = json!({
            "name": HOST_NAME,
            "description": "OriginKeep local provenance capture host",
            "path": host.display().to_string(),
            "type": "stdio",
            "allowed_origins": [format!("chrome-extension://{CHROMIUM_ID}/")]
        });
        let firefox = json!({
            "name": HOST_NAME,
            "description": "OriginKeep local provenance capture host",
            "path": host.display().to_string(),
            "type": "stdio",
            "allowed_extensions": [FIREFOX_ID]
        });
        let mut manifests = Vec::new();

        #[cfg(target_os = "macos")]
        let destinations: Vec<(PathBuf, &serde_json::Value)> = vec![
            (home.join("Library/Application Support/Google/Chrome/NativeMessagingHosts"), &chromium),
            (home.join("Library/Application Support/Chromium/NativeMessagingHosts"), &chromium),
            (home.join("Library/Application Support/Microsoft Edge/NativeMessagingHosts"), &chromium),
            (home.join("Library/Application Support/Mozilla/NativeMessagingHosts"), &firefox),
        ];

        #[cfg(target_os = "linux")]
        let destinations: Vec<(PathBuf, &serde_json::Value)> = vec![
            (home.join(".config/google-chrome/NativeMessagingHosts"), &chromium),
            (home.join(".config/chromium/NativeMessagingHosts"), &chromium),
            (home.join(".config/microsoft-edge/NativeMessagingHosts"), &chromium),
            (home.join(".mozilla/native-messaging-hosts"), &firefox),
        ];

        for (directory, manifest) in destinations {
            fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            let target = directory.join(format!("{HOST_NAME}.json"));
            fs::write(
                &target,
                serde_json::to_vec_pretty(manifest).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            manifests.push(target.display().to_string());
        }

        return Ok(BrowserSetupResult {
            platform: env::consts::OS.into(),
            native_host_path: host.display().to_string(),
            manifests,
            note: "Registered the bundled host for Chromium-family browsers and Firefox in the current user's profile. Safari uses local adoption/OS provenance because its WebExtensions downloads API is unavailable.".into(),
        });
    }

    #[allow(unreachable_code)]
    Err("Browser native-host registration is not implemented for this platform".into())
}

fn sibling_host() -> Result<PathBuf, String> {
    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let directory = executable
        .parent()
        .ok_or_else(|| "OriginKeep executable has no parent directory".to_string())?;
    #[cfg(target_os = "windows")]
    let name = "originkeep-native-host.exe";
    #[cfg(not(target_os = "windows"))]
    let name = "originkeep-native-host";
    Ok(directory.join(Path::new(name)))
}
