# OriginKeep Threat Model

This document covers the local-first OriginKeep desktop application, browser companion, native-message boundary, SQLite metadata store, remote freshness checks and recoverable archive.

## Assets to protect

- Local downloaded files and locally modified copies.
- Download provenance: source URLs, referrers, timestamps and filenames.
- SHA-256 fingerprints and version lineage.
- Remote-check evidence and HTTP validators.
- The SQLite database and lifecycle ledger.
- Recoverable archive copies.

## Trust boundaries

### Browser extension → native messaging host

Browser download metadata is untrusted input. The native host treats URLs, filenames and paths as data rather than executable instructions. Native messaging does not grant the extension permission to run arbitrary shell commands.

Release-package installs use a deterministic extension ID and a native-host manifest that allows exactly that extension origin. The NSIS installer registers the host for both Edge and Chrome under the current user's registry hive. Wildcard extension origins are not used.

### Native host / desktop → filesystem

Recorded paths may become stale, point to replaced files, or collide with files created after the original download. OriginKeep therefore re-hashes bytes immediately before archival and refuses to overwrite different bytes during restore.

### Desktop → remote source

Remote servers and redirects are untrusted. Freshness checks are explicit user actions and use bounded requests. Authentication failures remain `AUTH_REQUIRED`; network errors remain `CHECK_FAILED`; weak or absent validators never become a guessed `CURRENT` result.

Release builds disable automatic HTTP redirect following. Each redirect destination is parsed and validated before a new connection is made. DNS results are resolved before the request, every resolved address must be public, and those approved addresses are pinned into the per-hop HTTP client. Loopback, RFC1918/private, carrier-grade NAT, link-local, documentation, benchmark, multicast/reserved and IPv6 local/private destinations are rejected.

This protects the local machine and LAN from the recorded URL being used as a generic private-network request primitive. DNS rebinding risk is reduced by pinning the validated resolution into the request client for each hop.

### Application-data archive

The archive is local storage, not an independent backup service. A machine or disk failure can destroy both the original data and the archive. Users should not treat OriginKeep as their only backup for important files.

## Destructive-operation threats

### Deleting a locally edited file

Mitigation: archival requires the current SHA-256 to equal the immutable download fingerprint. A mismatch becomes `LOCAL_MODIFIED` and blocks archival.

### Partial copy followed by original deletion

Mitigation: OriginKeep copies first, flushes the destination, re-hashes it and compares the full SHA-256 before removing the original.

### Restore overwrites unrelated data

Mitigation: if the original path exists with a different SHA-256, restore fails. OriginKeep does not use a force-overwrite option.

### Crash during archive or restore

Mitigation: lifecycle state is persisted as `ARCHIVING` or `RESTORING` before filesystem mutation. Startup reconciliation checks which verified copy survives and chooses an evidence-backed state.

### Filename/path collision in the archive

Mitigation: archive names include the database record ID and a SHA-256 prefix in addition to a sanitized display filename. An existing archive path with different bytes is treated as a collision and the operation stops.

## Provenance threats

### Signed or expiring download URLs

OriginKeep keeps initiating/canonical source identity distinct from freshness evidence. A failed or expired final asset URL must not erase the original provenance record.

### Filename-based version guessing

OriginKeep does not infer equality or version lineage from names such as `final`, `(1)` or `new`. Exact equality requires SHA-256; source-family versioning uses canonical provenance plus content evidence.

### False freshness claims

Equal size alone does not prove `CURRENT`. A first metadata-only remote check establishes a baseline as `SOURCE_UNKNOWN`. `CURRENT` requires evidence such as HTTP 304 or an unchanged stored validator.

## Network and privacy threats

- Core features require no account or hosted OriginKeep backend.
- Local files are not uploaded for hashing, comparison, cleanup or restore.
- Freshness checks contact only a validated recorded public HTTP(S) source after an explicit user action.
- OriginKeep v0.1 does not collect browser cookies or persist authenticated web sessions.
- Remote responses are metadata evidence and are not interpreted as executable code by the lifecycle engine.
- Embedded URL credentials are rejected by the hardened freshness path.

See [`PRIVACY.md`](../PRIVACY.md) for the user-facing data-handling policy.

## Release supply-chain threats

GitHub Actions builds the Windows installer from repository source. Release workflows use least-privilege permissions needed for draft-release creation and artifact attestation. Third-party actions are pinned to full commit SHAs so the reviewed workflow does not silently follow a moved tag.

The Windows installer bundles `originkeep-native-host.exe` as a Tauri external binary and creates/removes the browser native-messaging registration through NSIS hooks. The release workflow also packages the companion extension from the same tagged source.

Artifact attestation is not Windows code signing and is not a guarantee that a binary is vulnerability-free. Public distribution should configure project-owned Windows signing credentials and review the draft installer before publication.

The repository must never commit private signing keys, certificate passwords, browser credentials or access tokens.

## Residual risks / v0.1 limitations

- The local archive shares the same machine and usually the same disk as the Downloads folder; it is recoverable cleanup, not disaster recovery.
- Filesystem permissions, antivirus software or concurrent external file changes can interrupt an operation. The lifecycle ledger is designed to detect and surface those cases rather than hide them.
- Authenticated/expiring remote sources may remain unverifiable without user-mediated access.
- PDF comparison covers extracted text layers only; it does not prove visual/layout equivalence.
- OriginKeep does not provide malware detection or sandbox untrusted documents.
- The deterministic companion ID is for the repository/release package. Browser-store publication must update `allowed_origins` if a store assigns a different ID.
- Unsigned Windows release candidates can still trigger Windows reputation warnings until project-owned code signing is configured.

## Security rule

When OriginKeep cannot prove that a destructive lifecycle operation is safe from locally available evidence, the operation must fail closed and preserve whichever verified copy still exists.
