# OriginKeep Threat Model

This document covers the local-first OriginKeep desktop application, browser companions, native-message boundary, SQLite/Passport metadata, remote freshness checks, optional trust verifiers and recoverable archive.

## Assets to protect

- Local tracked/adopted files and locally modified copies.
- Download/adoption provenance: source URLs, referrers, timestamps, filenames and optional page context.
- SHA-256 fingerprints, known locations and version lineage.
- User purpose, notes, expiry/review intent and portable Passport metadata.
- Remote-check evidence and HTTP validators.
- The SQLite database and lifecycle ledger.
- Recoverable archive copies.

## Trust boundaries

### Browser extension → native messaging host

Browser metadata is untrusted input. Native messages are length-bounded JSON; URLs, filenames, paths and optional context are data rather than executable instructions. Rich context text is bounded in the companion before Native Messaging.

Chromium Native Messaging uses an exact `allowed_origins` extension ID. Firefox uses its separate exact `allowed_extensions` add-on ID. Wildcards are not used. Windows NSIS owns Chrome/Edge/Firefox registry registration; macOS/Linux write browser-specific per-user manifests pointing to the bundled native host.

### Optional browser context

Broad HTTP(S) page access and `tabs` are optional rather than install-time requirements. The user must invoke the companion action to request them. A recent click context expires quickly and association is conservative. Revoking optional permissions stops future rich-page capture.

OriginKeep does not request cookies/history for context capture.

### Native host / desktop → filesystem

Recorded paths may become stale, point to replaced files, or collide with files created after the original download. OriginKeep re-hashes bytes before archival and refuses to overwrite different bytes during restore.

Move/rename relinking is content-based: explicit candidates and bounded search results must exactly match the immutable SHA-256. Directory discovery is bounded by count/depth and does not traverse symlink directories.

### Portable Passport JSON → local database

Portable `.originkeep.json` files are untrusted metadata. Import requires a supported format/version and recomputes the selected local file's SHA-256 before reconnecting metadata. Filename similarity is insufficient.

The portable Passport is not itself a publisher signature. An attacker can create a new JSON document for arbitrary bytes; authenticity claims therefore remain separate Trust Lens evidence.

Portable exports exclude local/archived absolute paths and credentials. Shareable URL metadata has a stricter secret-redaction boundary than the local provenance database.

### Desktop → remote source

Remote servers and redirects are untrusted. Freshness checks are explicit, bounded and anonymous. Authentication failures remain `AUTH_REQUIRED`; network errors remain `CHECK_FAILED`; weak/absent validators never become guessed `CURRENT`.

Automatic redirects are disabled. Each redirect destination is parsed and validated before a new connection. DNS results are resolved first, every address must be public, and approved addresses are pinned into the request client. Loopback, RFC1918/private, carrier-grade NAT, link-local, documentation, benchmark, multicast/reserved and IPv6 local/private destinations are rejected.

This reduces SSRF/private-network and DNS-rebinding risk from recorded URLs.

### Trust Lens → local verifier tools

OriginKeep treats verifier output as evidence rather than authority over application control flow.

- C2PA: an installed `c2patool` may parse/validate manifests. Without it, marker detection is explicitly unverified.
- Sigstore: `cosign verify-blob` requires an adjacent bundle plus user-provided expected certificate identity and OIDC issuer. A bundle's existence alone never becomes `VERIFIED`.
- Platform signature/origin tools are reported as platform evidence, not malware/truth verdicts.

File paths and verifier parameters are passed as process arguments instead of concatenated shell commands. Optional external tools may have their own network/trust-root behavior and must not receive secrets invented/extracted by OriginKeep.

### Application-data archive

The archive is local storage, not an independent backup service. Machine/disk failure can destroy both originals and archive copies.

## Destructive-operation threats

### Deleting a locally edited file

Mitigation: archival requires current SHA-256 to equal the immutable fingerprint. A mismatch becomes `LOCAL_MODIFIED` and blocks archival.

### Intent causes unintended deletion

Mitigation: Passport intents (`ARCHIVE_WHEN_EXPIRED`, `ARCHIVE_WHEN_SUPERSEDED`, etc.) only change review recommendations. They do not execute archive/delete actions automatically. `NEVER_ARCHIVE` overrides generic candidate selection.

### Partial copy followed by original deletion

Mitigation: copy first, flush destination, re-hash it, then remove original only after full SHA-256 equality.

### Restore overwrites unrelated data

Mitigation: if the original path exists with different SHA-256, restore fails. No force-overwrite option exists.

### Crash during archive or restore

Mitigation: lifecycle state is persisted as `ARCHIVING`/`RESTORING` before mutation; startup reconciliation checks surviving verified copies.

### Filename/path collision in archive

Mitigation: archive names include database ID + SHA prefix + sanitized display name. Existing different bytes stop the operation.

## Provenance threats

### Signed or expiring download URLs

Initiating/canonical source identity remains distinct from freshness evidence. Failed/expired final URLs do not erase original provenance.

### Credential-bearing URL query parameters

The local database may preserve browser-reported URLs for provenance/source identity. Portable/shareable Passport export must redact common credential-bearing query keys rather than blindly exporting secrets. Unknown query parameters are preserved because they can be semantically significant resource identity.

### Filename-based version guessing

OriginKeep never infers equality/version lineage from names such as `final`, `(1)` or `new`. Equality requires SHA-256; source-family versioning uses provenance + content evidence.

### False freshness claims

Equal size alone does not prove `CURRENT`. A first metadata-only check establishes `SOURCE_UNKNOWN`; `CURRENT` needs deterministic evidence such as HTTP 304 or an unchanged validator.

### False authenticity claims

C2PA cryptographic validation does not automatically mean an issuer is trusted or content is true. Sigstore verification must bind an artifact to the expected identity/issuer. Platform origin metadata can be absent, lost or copied. Trust Lens therefore shows independent evidence states and never computes a global trust score.

## Network and privacy threats

- No OriginKeep account/backend is required.
- Local file contents are not uploaded for hashing, comparison, Passport operations, cleanup or restore.
- Freshness checks contact only validated public HTTP(S) sources after explicit action.
- Browser cookies/auth sessions are not collected/replayed.
- Remote responses are evidence, not executable lifecycle instructions.
- Embedded URL credentials are rejected by the hardened freshness path.
- Rich page context is opt-in, bounded and stored locally.

See [`PRIVACY.md`](../PRIVACY.md).

## Release supply-chain threats

GitHub Actions build desktop artifacts from repository source using frozen npm/Cargo lockfiles. Third-party actions are pinned to full commit SHAs. Version tags must point to commits already contained in `main` before release creation.

Release CI builds Windows NSIS, macOS DMG, Linux AppImage/DEB and separate Chromium/Firefox companion archives. The same target-triple sidecar mechanism is exercised by platform bundle jobs.

Artifact attestation is not operating-system code signing, notarization or a vulnerability-free guarantee. Public distribution should configure project-owned signing/notarization credentials and complete clean-machine review before publishing a draft release.

Never commit private signing keys, certificate passwords, browser credentials or access tokens.

## Residual limitations

- Local archive is recoverable cleanup, not disaster recovery.
- Filesystem permissions, security software or concurrent external changes can interrupt operations; the ledger surfaces rather than hides them.
- Authenticated/expiring sources can remain unverifiable without user-mediated access.
- PDF comparison covers extracted text layers, not visual/layout equivalence.
- OriginKeep is not malware detection/sandboxing.
- Browser-store Chromium IDs may differ from the deterministic repository package and require an allowlist update.
- Unsigned Windows/macOS release candidates can trigger platform security/reputation warnings.
- Safari automatic download provenance parity is not claimed because Apple's current WebExtension tooling does not support the required `downloads` capability.
- Optional C2PA/Sigstore verification is only as available/correct as the installed verifier tooling and user-provided trust expectations.

## Security rule

When OriginKeep cannot prove that a destructive lifecycle operation is safe from locally available evidence, the operation must fail closed and preserve whichever verified copy still exists.
