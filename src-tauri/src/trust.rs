use crate::{passport, storage};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Output},
};

const HEURISTIC_WINDOW: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustEvidence {
    pub state: String,
    pub summary: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustLens {
    pub download_id: i64,
    pub file_name: String,
    pub integrity: TrustEvidence,
    pub origin: TrustEvidence,
    pub platform_origin: TrustEvidence,
    pub publisher_signature: TrustEvidence,
    pub c2pa: TrustEvidence,
    pub sigstore: TrustEvidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SigstoreVerification {
    pub download_id: i64,
    pub bundle_path: String,
    pub identity: String,
    pub issuer: String,
    pub state: String,
    pub evidence: String,
}

pub fn inspect(path: &Path, download_id: i64) -> Result<TrustLens, String> {
    passport::initialize_database(path)?;
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let (file_name, local_path, expected_hash, original_url, source_identity): (
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT file_name, local_path, sha256, original_url, source_identity FROM downloads WHERE id = ?1",
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(|error| error.to_string())?;
    let file_path = PathBuf::from(local_path);

    let integrity = integrity_evidence(&file_path, expected_hash.as_deref());
    let origin = if source_identity.is_some() || !original_url.trim().is_empty() {
        TrustEvidence {
            state: "RECORDED".into(),
            summary: "OriginKeep has recorded download provenance for this file.".into(),
            detail: Some(source_identity.unwrap_or(original_url)),
        }
    } else {
        TrustEvidence {
            state: "UNKNOWN".into(),
            summary: "No source provenance is recorded.".into(),
            detail: None,
        }
    };

    Ok(TrustLens {
        download_id,
        file_name,
        integrity,
        origin,
        platform_origin: platform_origin_evidence(&file_path),
        publisher_signature: publisher_signature_evidence(&file_path),
        c2pa: c2pa_evidence(&file_path),
        sigstore: sigstore_bundle_evidence(&file_path),
    })
}

fn integrity_evidence(file_path: &Path, expected_hash: Option<&str>) -> TrustEvidence {
    let Some(expected_hash) = expected_hash else {
        return TrustEvidence {
            state: "UNAVAILABLE".into(),
            summary: "No download-time SHA-256 fingerprint is stored.".into(),
            detail: None,
        };
    };
    if !file_path.is_file() {
        return TrustEvidence {
            state: "LOCAL_MISSING".into(),
            summary: "The tracked local file is not present at its current path.".into(),
            detail: Some(file_path.display().to_string()),
        };
    }
    match storage::sha256_file(file_path) {
        Ok(current) if current == expected_hash => TrustEvidence {
            state: "MATCH".into(),
            summary: "The current bytes match OriginKeep's immutable download-time SHA-256.".into(),
            detail: Some(current),
        },
        Ok(current) => TrustEvidence {
            state: "LOCAL_MODIFIED".into(),
            summary: "The current bytes do not match the recorded download fingerprint.".into(),
            detail: Some(format!("expected {expected_hash}; current {current}")),
        },
        Err(error) => TrustEvidence {
            state: "UNREADABLE".into(),
            summary: "OriginKeep could not read the file to verify its fingerprint.".into(),
            detail: Some(error.to_string()),
        },
    }
}

#[cfg(target_os = "windows")]
fn platform_origin_evidence(file_path: &Path) -> TrustEvidence {
    let zone = PathBuf::from(format!("{}:Zone.Identifier", file_path.display()));
    match fs::read_to_string(&zone) {
        Ok(content) => TrustEvidence {
            state: "WINDOWS_MOTW_PRESENT".into(),
            summary: "Windows Mark of the Web metadata is present.".into(),
            detail: Some(content.chars().take(2000).collect()),
        },
        Err(_) => TrustEvidence {
            state: "NOT_RECORDED_BY_PLATFORM".into(),
            summary: "No readable Windows Zone.Identifier stream was found.".into(),
            detail: None,
        },
    }
}

#[cfg(target_os = "macos")]
fn platform_origin_evidence(file_path: &Path) -> TrustEvidence {
    let output = Command::new("mdls")
        .args(["-raw", "-name", "kMDItemWhereFroms"])
        .arg(file_path)
        .output();
    command_evidence(
        output,
        "MACOS_WHEREFROMS_PRESENT",
        "macOS download provenance metadata was found.",
        "NOT_RECORDED_BY_PLATFORM",
        "macOS did not return kMDItemWhereFroms provenance for this file.",
    )
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn platform_origin_evidence(_file_path: &Path) -> TrustEvidence {
    TrustEvidence {
        state: "NOT_APPLICABLE".into(),
        summary:
            "This platform has no OriginKeep-supported built-in download-origin metadata source."
                .into(),
        detail: None,
    }
}

#[cfg(target_os = "windows")]
fn publisher_signature_evidence(file_path: &Path) -> TrustEvidence {
    let script = "$s=Get-AuthenticodeSignature -LiteralPath $args[0]; $subject=if($s.SignerCertificate){$s.SignerCertificate.Subject}else{''}; Write-Output ($s.Status.ToString()+'|'+$subject+'|'+$s.StatusMessage)";
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .arg(file_path)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let state = if text.starts_with("Valid|") {
                "VALID"
            } else if text.starts_with("NotSigned|") {
                "NOT_SIGNED"
            } else {
                "UNVERIFIED"
            };
            TrustEvidence {
                state: state.into(),
                summary: if state == "VALID" {
                    "Windows reports a valid Authenticode publisher signature.".into()
                } else {
                    "Windows did not report a valid Authenticode publisher signature.".into()
                },
                detail: nonempty(text),
            }
        }
        Ok(output) => TrustEvidence {
            state: "CHECK_FAILED".into(),
            summary: "Windows publisher-signature inspection failed.".into(),
            detail: command_detail(&output),
        },
        Err(error) => TrustEvidence {
            state: "TOOL_UNAVAILABLE".into(),
            summary: "PowerShell is unavailable for Authenticode inspection.".into(),
            detail: Some(error.to_string()),
        },
    }
}

#[cfg(target_os = "macos")]
fn publisher_signature_evidence(file_path: &Path) -> TrustEvidence {
    let output = Command::new("codesign")
        .args(["--verify", "--deep", "--strict", "--verbose=2"])
        .arg(file_path)
        .output();
    command_evidence(
        output,
        "VALID",
        "macOS codesign verification passed for this executable/app artifact.",
        "NOT_VERIFIED",
        "macOS codesign verification did not pass or is not applicable to this file.",
    )
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn publisher_signature_evidence(_file_path: &Path) -> TrustEvidence {
    TrustEvidence {
        state: "NOT_APPLICABLE".into(),
        summary: "No platform publisher-signature verifier is configured for this file on Linux."
            .into(),
        detail: None,
    }
}

fn c2pa_evidence(file_path: &Path) -> TrustEvidence {
    if !file_path.is_file() {
        return TrustEvidence {
            state: "LOCAL_MISSING".into(),
            summary: "C2PA inspection requires the local file.".into(),
            detail: None,
        };
    }
    match Command::new("c2patool").arg(file_path).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parsed = serde_json::from_str::<Value>(&stdout);
            match parsed {
                Ok(value)
                    if value.get("active_manifest").is_some()
                        || value.get("activeManifest").is_some() =>
                {
                    let invalid = validation_has_errors(&value);
                    TrustEvidence {
                        state: if invalid {
                            "INVALID"
                        } else {
                            "CRYPTOGRAPHIC_VALIDATION_PASSED"
                        }
                        .into(),
                        summary: if invalid {
                            "C2PA metadata is present, but c2patool reported validation problems."
                                .into()
                        } else {
                            "C2PA manifest validation passed. This proves cryptographic consistency, not that the issuer is inherently trustworthy.".into()
                        },
                        detail: Some(stdout.chars().take(4000).collect()),
                    }
                }
                Ok(_) => TrustEvidence {
                    state: "NO_ACTIVE_MANIFEST".into(),
                    summary: "c2patool found no active C2PA manifest.".into(),
                    detail: None,
                },
                Err(_) => TrustEvidence {
                    state: "CHECK_FAILED".into(),
                    summary: "c2patool returned output that OriginKeep could not parse as JSON."
                        .into(),
                    detail: Some(stdout.chars().take(2000).collect()),
                },
            }
        }
        Ok(output) => TrustEvidence {
            state: "NOT_VERIFIED".into(),
            summary: "c2patool did not validate a C2PA manifest for this file.".into(),
            detail: command_detail(&output),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if heuristic_contains_c2pa(file_path) {
                TrustEvidence {
                    state: "PRESENT_UNVERIFIED".into(),
                    summary: "The file contains C2PA/JUMBF marker bytes, but c2patool is not installed, so OriginKeep does not claim cryptographic verification.".into(),
                    detail: None,
                }
            } else {
                TrustEvidence {
                    state: "NOT_DETECTED_HEURISTIC".into(),
                    summary: "No C2PA/JUMBF marker was found in OriginKeep's bounded heuristic scan. Install c2patool for authoritative verification.".into(),
                    detail: None,
                }
            }
        }
        Err(error) => TrustEvidence {
            state: "TOOL_UNAVAILABLE".into(),
            summary: "C2PA verification could not start.".into(),
            detail: Some(error.to_string()),
        },
    }
}

fn validation_has_errors(value: &Value) -> bool {
    ["validation_status", "validationStatus"]
        .iter()
        .filter_map(|key| value.get(*key))
        .any(|status| match status {
            Value::Array(items) => !items.is_empty(),
            Value::Null => false,
            other => !other.to_string().is_empty(),
        })
}

fn heuristic_contains_c2pa(file_path: &Path) -> bool {
    let Ok(mut file) = File::open(file_path) else {
        return false;
    };
    let Ok(length) = file.metadata().map(|value| value.len()) else {
        return false;
    };
    let mut windows = Vec::new();
    let first_len = length.min(HEURISTIC_WINDOW) as usize;
    let mut first = vec![0_u8; first_len];
    if file.read_exact(&mut first).is_ok() {
        windows.push(first);
    }
    if length > HEURISTIC_WINDOW {
        let tail_len = length.min(HEURISTIC_WINDOW) as usize;
        if file.seek(SeekFrom::End(-(tail_len as i64))).is_ok() {
            let mut tail = vec![0_u8; tail_len];
            if file.read_exact(&mut tail).is_ok() {
                windows.push(tail);
            }
        }
    }
    windows.iter().any(|bytes| {
        bytes
            .windows(4)
            .any(|window| window.eq_ignore_ascii_case(b"c2pa"))
            || bytes
                .windows(5)
                .any(|window| window.eq_ignore_ascii_case(b"jumbf"))
    })
}

fn sigstore_bundle_evidence(file_path: &Path) -> TrustEvidence {
    match find_sigstore_bundle(file_path) {
        Some(bundle) => TrustEvidence {
            state: "BUNDLE_PRESENT".into(),
            summary: "A Sigstore bundle is adjacent to this artifact. Verification still requires the expected signer identity and OIDC issuer.".into(),
            detail: Some(bundle.display().to_string()),
        },
        None => TrustEvidence {
            state: "NOT_FOUND".into(),
            summary: "No adjacent Sigstore bundle was found.".into(),
            detail: None,
        },
    }
}

pub fn verify_sigstore(
    path: &Path,
    download_id: i64,
    identity: String,
    issuer: String,
) -> Result<SigstoreVerification, String> {
    passport::initialize_database(path)?;
    let identity = identity.trim().to_string();
    let issuer = issuer.trim().to_string();
    if identity.is_empty() || issuer.is_empty() {
        return Err(
            "Sigstore verification requires the expected certificate identity and OIDC issuer"
                .into(),
        );
    }
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let local_path: String = connection
        .query_row(
            "SELECT local_path FROM downloads WHERE id = ?1",
            [download_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let file_path = PathBuf::from(local_path);
    if !file_path.is_file() {
        return Err("Sigstore verification requires the local artifact".into());
    }
    let bundle = find_sigstore_bundle(&file_path)
        .ok_or_else(|| "No adjacent .sigstore.json bundle was found".to_string())?;
    let output = Command::new("cosign")
        .arg("verify-blob")
        .arg(&file_path)
        .arg("--bundle")
        .arg(&bundle)
        .arg(format!("--certificate-identity={identity}"))
        .arg(format!("--certificate-oidc-issuer={issuer}"))
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "Cosign is not installed. OriginKeep will not claim Sigstore verification without the official verifier.".to_string()
            } else {
                error.to_string()
            }
        })?;
    if !output.status.success() {
        return Ok(SigstoreVerification {
            download_id,
            bundle_path: bundle.display().to_string(),
            identity,
            issuer,
            state: "VERIFICATION_FAILED".into(),
            evidence: command_detail(&output)
                .unwrap_or_else(|| "Cosign returned a non-zero status.".into()),
        });
    }
    Ok(SigstoreVerification {
        download_id,
        bundle_path: bundle.display().to_string(),
        identity,
        issuer,
        state: "VERIFIED".into(),
        evidence: String::from_utf8_lossy(&output.stdout)
            .trim()
            .chars()
            .take(4000)
            .collect(),
    })
}

fn find_sigstore_bundle(file_path: &Path) -> Option<PathBuf> {
    let direct = PathBuf::from(format!("{}.sigstore.json", file_path.display()));
    if direct.is_file() {
        return Some(direct);
    }
    let stem = file_path.file_stem()?.to_string_lossy();
    let sibling = file_path.with_file_name(format!("{stem}.sigstore.json"));
    sibling.is_file().then_some(sibling)
}

fn command_evidence(
    output: Result<Output, std::io::Error>,
    success_state: &str,
    success_summary: &str,
    failure_state: &str,
    failure_summary: &str,
) -> TrustEvidence {
    match output {
        Ok(output) if output.status.success() => TrustEvidence {
            state: success_state.into(),
            summary: success_summary.into(),
            detail: command_detail(&output),
        },
        Ok(output) => TrustEvidence {
            state: failure_state.into(),
            summary: failure_summary.into(),
            detail: command_detail(&output),
        },
        Err(error) => TrustEvidence {
            state: "TOOL_UNAVAILABLE".into(),
            summary: failure_summary.into(),
            detail: Some(error.to_string()),
        },
    }
}

fn command_detail(output: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    nonempty(format!(
        "{}{}{}",
        stdout.trim(),
        if stdout.trim().is_empty() || stderr.trim().is_empty() {
            ""
        } else {
            " | "
        },
        stderr.trim()
    ))
    .map(|value| value.chars().take(4000).collect())
}

fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_file() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("originkeep-trust-{unique}.bin"))
    }

    #[test]
    fn integrity_is_deterministic() {
        let path = temp_file();
        fs::write(&path, b"trust lens").unwrap();
        let hash = storage::sha256_file(&path).unwrap();
        assert_eq!(integrity_evidence(&path, Some(&hash)).state, "MATCH");
        fs::write(&path, b"changed").unwrap();
        assert_eq!(
            integrity_evidence(&path, Some(&hash)).state,
            "LOCAL_MODIFIED"
        );
        fs::remove_file(path).ok();
    }

    #[test]
    fn sigstore_bundle_detection_is_adjacent_and_explicit() {
        let path = temp_file();
        fs::write(&path, b"artifact").unwrap();
        let bundle = PathBuf::from(format!("{}.sigstore.json", path.display()));
        fs::write(&bundle, b"{}").unwrap();
        assert_eq!(
            find_sigstore_bundle(&path).as_deref(),
            Some(bundle.as_path())
        );
        fs::remove_file(path).ok();
        fs::remove_file(bundle).ok();
    }
}
