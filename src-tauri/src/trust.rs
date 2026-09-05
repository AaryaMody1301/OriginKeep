use crate::{passport, storage};
use c2pa::{Reader, ValidationState};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use sigstore_trust_root::{TrustedRoot, SIGSTORE_PRODUCTION_TRUSTED_ROOT};
use sigstore_types::{Bundle, Sha256Hash};
use sigstore_verify::{VerificationPolicy, Verifier};
use std::{fs, path::{Path, PathBuf}};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustEvidence {
    pub kind: String,
    pub state: String,
    pub summary: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustReport {
    pub download_id: i64,
    pub file_name: String,
    pub evidence: Vec<TrustEvidence>,
}

pub fn inspect(path: &Path, download_id: i64) -> Result<TrustReport, String> {
    passport::initialize_database(path)?;
    let passport = passport::get_passport(path, download_id)?;
    let local_path = PathBuf::from(&passport.local_path);
    let mut evidence = Vec::new();

    evidence.push(origin_evidence(&passport));
    evidence.push(integrity_evidence(&passport, &local_path));
    evidence.push(remote_evidence(path, download_id)?);
    evidence.push(os_evidence(&passport));
    evidence.push(c2pa_evidence(&local_path));
    evidence.push(sigstore_evidence(&passport, &local_path));

    Ok(TrustReport {
        download_id,
        file_name: passport.file_name,
        evidence,
    })
}

fn origin_evidence(passport: &passport::FilePassport) -> TrustEvidence {
    let mut details = vec![format!("Recorded origin: {}", passport.original_url)];
    if let Some(page) = passport.page_url.as_ref() {
        details.push(format!("Download context page: {page}"));
    }
    if let Some(referrer) = passport.referrer.as_ref() {
        details.push(format!("Browser referrer: {referrer}"));
    }
    TrustEvidence {
        kind: "ORIGIN".into(),
        state: if passport.source_identity.is_some() { "RECORDED" } else { "PARTIAL" }.into(),
        summary: "OriginKeep preserves browser/source evidence without treating it as publisher authentication.".into(),
        details,
    }
}

fn integrity_evidence(passport: &passport::FilePassport, local_path: &Path) -> TrustEvidence {
    match (passport.sha256.as_deref(), local_path.is_file()) {
        (Some(expected), true) => match storage::sha256_file(local_path) {
            Ok(current) if current == expected => TrustEvidence {
                kind: "INTEGRITY".into(),
                state: "MATCH".into(),
                summary: "Current local bytes match the immutable download-time SHA-256.".into(),
                details: vec![expected.to_string()],
            },
            Ok(current) => TrustEvidence {
                kind: "INTEGRITY".into(),
                state: "LOCAL_MODIFIED".into(),
                summary: "Current local bytes differ from the recorded download-time fingerprint.".into(),
                details: vec![format!("expected {expected}"), format!("current {current}")],
            },
            Err(error) => TrustEvidence {
                kind: "INTEGRITY".into(),
                state: "UNAVAILABLE".into(),
                summary: "OriginKeep could not read the local file to verify its fingerprint.".into(),
                details: vec![error.to_string()],
            },
        },
        (Some(expected), false) => TrustEvidence {
            kind: "INTEGRITY".into(),
            state: "LOCAL_MISSING".into(),
            summary: "The recorded SHA-256 remains available, but the current local path is missing.".into(),
            details: vec![expected.to_string()],
        },
        (None, _) => TrustEvidence {
            kind: "INTEGRITY".into(),
            state: "UNKNOWN".into(),
            summary: "No immutable SHA-256 was recorded for this item.".into(),
            details: Vec::new(),
        },
    }
}

fn remote_evidence(path: &Path, download_id: i64) -> Result<TrustEvidence, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    let row: Option<(String, String, Option<i64>, String)> = connection
        .query_row(
            r#"
            SELECT result_state, checked_at, http_status, evidence
            FROM remote_checks
            WHERE download_id = ?1
            ORDER BY id DESC LIMIT 1
            "#,
            [download_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    Ok(match row {
        Some((state, checked_at, status, explanation)) => TrustEvidence {
            kind: "REMOTE_SOURCE".into(),
            state,
            summary: explanation,
            details: vec![
                format!("Checked: {checked_at}"),
                status.map(|value| format!("HTTP {value}")).unwrap_or_else(|| "No HTTP status".into()),
            ],
        },
        None => TrustEvidence {
            kind: "REMOTE_SOURCE".into(),
            state: "NOT_CHECKED".into(),
            summary: "No remote freshness evidence has been collected yet.".into(),
            details: Vec::new(),
        },
    })
}

fn os_evidence(passport: &passport::FilePassport) -> TrustEvidence {
    match passport.os_provenance.as_ref() {
        Some(value) => TrustEvidence {
            kind: "OS_PROVENANCE".into(),
            state: "RECORDED".into(),
            summary: "Operating-system provenance evidence was imported locally.".into(),
            details: vec![value.clone()],
        },
        None => TrustEvidence {
            kind: "OS_PROVENANCE".into(),
            state: "NOT_IMPORTED".into(),
            summary: "OS-level provenance has not been imported for this file.".into(),
            details: Vec::new(),
        },
    }
}

fn c2pa_evidence(local_path: &Path) -> TrustEvidence {
    if !local_path.is_file() {
        return TrustEvidence {
            kind: "C2PA".into(),
            state: "LOCAL_MISSING".into(),
            summary: "C2PA cannot be inspected while the local file is missing.".into(),
            details: Vec::new(),
        };
    }
    match Reader::default().with_file(local_path) {
        Ok(reader) => {
            let state = match reader.validation_state() {
                ValidationState::Trusted => "TRUSTED",
                ValidationState::Valid => "VALID_UNTRUSTED",
                ValidationState::Invalid => "INVALID",
            };
            let mut details = Vec::new();
            if let Some(manifest) = reader.active_manifest() {
                if let Some(value) = manifest.issuer() {
                    details.push(format!("Issuer: {value}"));
                }
                if let Some(value) = manifest.common_name() {
                    details.push(format!("Certificate common name: {value}"));
                }
                if let Some(value) = manifest.time() {
                    details.push(format!("Signed: {value}"));
                }
                if let Some(value) = manifest.claim_generator() {
                    details.push(format!("Claim generator: {value}"));
                }
            }
            TrustEvidence {
                kind: "C2PA".into(),
                state: state.into(),
                summary: match state {
                    "TRUSTED" => "C2PA provenance is cryptographically valid and chains to the configured trust anchors.",
                    "VALID_UNTRUSTED" => "C2PA integrity is valid, but OriginKeep does not have a trusted chain for the signer.",
                    _ => "C2PA data is present but failed validation.",
                }
                .into(),
                details,
            }
        }
        Err(error) => {
            let message = error.to_string();
            let lower = message.to_ascii_lowercase();
            let absent = lower.contains("not found")
                || lower.contains("no manifest")
                || lower.contains("jumbf") && lower.contains("missing");
            TrustEvidence {
                kind: "C2PA".into(),
                state: if absent { "NOT_PRESENT" } else { "UNAVAILABLE" }.into(),
                summary: if absent {
                    "No readable C2PA manifest was found for this asset."
                } else {
                    "OriginKeep could not parse C2PA provenance for this asset."
                }
                .into(),
                details: vec![message],
            }
        }
    }
}

fn sigstore_evidence(passport: &passport::FilePassport, local_path: &Path) -> TrustEvidence {
    let bundle_path = PathBuf::from(format!("{}.sigstore.json", local_path.display()));
    if !bundle_path.is_file() {
        return TrustEvidence {
            kind: "SIGSTORE".into(),
            state: "NOT_PRESENT".into(),
            summary: "No adjacent Sigstore bundle (.sigstore.json) was found.".into(),
            details: Vec::new(),
        };
    }
    let Some(hash) = passport.sha256.as_deref() else {
        return TrustEvidence {
            kind: "SIGSTORE".into(),
            state: "UNKNOWN".into(),
            summary: "A Sigstore bundle is present, but OriginKeep has no SHA-256 for the artifact.".into(),
            details: Vec::new(),
        };
    };
    let verification = (|| -> Result<sigstore_verify::VerificationResult, String> {
        let json = fs::read_to_string(&bundle_path).map_err(|error| error.to_string())?;
        let bundle = Bundle::from_json(&json).map_err(|error| error.to_string())?;
        let root = TrustedRoot::from_json(SIGSTORE_PRODUCTION_TRUSTED_ROOT)
            .map_err(|error| error.to_string())?;
        let digest = Sha256Hash::from_hex(hash).map_err(|error| error.to_string())?;
        let verifier = Verifier::new(&root);
        verifier
            .verify(digest, &bundle, &VerificationPolicy::default())
            .map_err(|error| error.to_string())
    })();
    match verification {
        Ok(result) => {
            let mut details = Vec::new();
            if let Some(identity) = result.identity {
                details.push(format!("Identity: {identity}"));
            }
            if let Some(issuer) = result.issuer {
                details.push(format!("Issuer: {issuer}"));
            }
            if let Some(time) = result.integrated_time {
                details.push(format!("Transparency-log integrated time: {time}"));
            }
            details.extend(result.warnings.into_iter().map(|warning| format!("Warning: {warning}")));
            TrustEvidence {
                kind: "SIGSTORE".into(),
                state: if result.success { "VERIFIED" } else { "INVALID" }.into(),
                summary: if result.success {
                    "The Sigstore bundle cryptographically verifies against the recorded artifact SHA-256 and embedded production trust root."
                } else {
                    "Sigstore verification completed but did not establish a valid signature."
                }
                .into(),
                details,
            }
        }
        Err(error) => TrustEvidence {
            kind: "SIGSTORE".into(),
            state: "INVALID".into(),
            summary: "The adjacent Sigstore bundle could not be cryptographically verified.".into(),
            details: vec![error],
        },
    }
}
