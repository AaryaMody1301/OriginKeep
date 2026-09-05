# OriginKeep Threat Model

This document covers the local-first OriginKeep desktop application, browser companions, native-message boundary, SQLite metadata store, File Passports, remote freshness checks, Trust Lens and recoverable archive.

## Assets to protect

- Local downloaded/adopted files and locally modified copies.
- Provenance: source URLs, referrers, timestamps, filenames and matched browser context.
- User notes, purpose, review/expiry values and retention intent.
- SHA-256 fingerprints, location history and version lineage.
- Portable `.originkeep.json` passports.
- C2PA/Sigstore/OS-provenance evidence and remote-check evidence.
- The SQLite database and lifecycle ledger.
- Recoverable archive copies.

## Trust boundaries

### Browser companion → native messaging host

Browser metadata and matched page context are untrusted input. The native host treats URLs, filenames, page text and paths as data rather than executable instructions. Native messaging does not grant the companion permission to run arbitrary shell commands.

Chromium manifests use exact `allowed_origins`; Firefox manifests use exact `allowed_extensions`. Wildcards are not used. Windows NSIS registers Chrome, Edge and Firefox under the current user's registry hive. macOS/Linux registration writes per-user manifests pointing at the bundled native host.

The browser context pipeline deliberately has a narrow lifetime: a bounded recent list is kept in session-only storage where available, with memory fallback, and only a matched download context is persisted in OriginKeep.

### Native host / desktop → filesystem

Recorded paths may become stale, point to replaced files, or collide with files created after the original download. OriginKeep therefore hashes bytes before identity-sensitive operations.

- archive re-verifies the immutable recorded SHA-256;
- restore refuses to overwrite different bytes;
- relink requires an exact SHA-256 match;
- move scanning uses size only as a prefilter and exact SHA-256 as the identity decision;
- local adoption computes SHA-256 before the file enters the Passport model.

### Portable passport → local database

A portable `.originkeep.json` file is untrusted metadata. It does not establish identity by filename, path, claimed version or source URL.

Import requires the adjacent asset to match the passport's recorded SHA-256 before the metadata is ingested. The receiving database applies its own duplicate/source-family rules rather than blindly trusting a foreign version number.

Portable passports intentionally omit absolute local paths/location history, but can still contain sensitive source/referrer/page URLs, query strings, page context and user notes. Sharing a passport is therefore an explicit export action rather than an automatic sync feature.

### Desktop → remote source

Remote servers and redirects are untrusted. Freshness checks are explicit user actions and use bounded requests. Authentication failures remain `AUTH_REQUIRED`; network errors remain `CHECK_FAILED`; weak or absent validators never become a guessed `CURRENT` result.

Release builds disable automatic HTTP redirect following. Each redirect destination is parsed and validated before a new connection is made. DNS results are resolved before the request, every resolved address must be public, and those approved addresses are pinned into the per-hop HTTP client. Loopback, RFC1918/private, carrier-grade NAT, link-local, documentation, benchmark, multicast/reserved and IPv6 local/private destinations are rejected.

This protects the local machine and LAN from the recorded URL being used as a generic private-network request primitive. DNS rebinding risk is reduced by pinning the validated resolution into the request client for each hop.

OriginKeep does not replay browser cookies, authorization headers or authenticated sessions. A protected source can remain `AUTH_REQUIRED` rather than weakening this boundary.

### Trust Lens → user interpretation

Cryptographic provenance is evidence, not a safety verdict.

- SHA-256 proves byte identity, not benign intent.
- C2PA can prove cryptographic provenance assertions; `VALID_UNTRUSTED` is not the same as a trusted signer.
- Sigstore verification proves that the adjacent bundle verifies for the recorded digest under the configured trust-root snapshot/policy. It is not malware scanning.
- OS provenance can indicate where an operating system believes a file came from, but it can be absent, stripped, copied or manipulated.
- source/referrer evidence records provenance but does not authenticate a publisher.

The UI must keep these signals independent and must not derive an unexplained global "safe" score.

The Sigstore implementation uses an embedded production trusted-root snapshot from the dependency graph to avoid a hidden network dependency in Trust Lens. A shipped snapshot can become stale; an inability to verify with it must not be presented as evidence that an artifact is malicious.

### Application-data archive

The archive is local storage, not an independent backup service. A machine or disk failure can destroy both the original data and the archive. Users should not treat OriginKeep as their only backup for important files.

## Destructive-operation threats

### Deleting/archiving a locally edited file

Mitigation: archival requires the current SHA-256 to equal the immutable download/adoption fingerprint. A mismatch becomes `LOCAL_MODIFIED` and blocks archival.

Retention intent is metadata in OriginKeep 2.0. It does not independently execute a destructive action.

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

### Filename-based identity/version guessing

OriginKeep does not infer equality or version lineage from names such as `final`, `(1)` or `new`. Exact equality requires SHA-256; source-family versioning uses canonical provenance plus content evidence.

Location recovery also requires SHA-256, so a renamed unrelated file is not adopted into a tracked identity merely because its name resembles the missing record.

### False freshness claims

Equal size alone does not prove `CURRENT`. A first metadata-only remote check establishes a baseline as `SOURCE_UNKNOWN`. `CURRENT` requires evidence such as HTTP 304 or an unchanged stored validator.

### Browser page-context misassociation

A click can occur near another download or a page can issue downloads indirectly. Mitigations:

- short two-minute matching window;
- exact download/final URL match is preferred;
- referrer/page match is a fallback only;
- matched candidates are consumed;
- context is shown as context evidence, not as cryptographic origin proof.

## Local scan threats

Move scanning can be expensive on large directory trees. Mitigations:

- scan root is explicitly supplied by the user;
- scan has a hard file-count cap;
- unreadable directories/files are skipped rather than escalated;
- size filters avoid unnecessary hashing where possible;
- file contents remain local.

## C2PA/Sigstore parser threats

Provenance/signature metadata is untrusted structured input. The libraries parse local files/bundles inside the OriginKeep process. This carries normal parser/library risk.

Mitigations include:

- dependency versions are locked;
- CI runs strict Clippy/tests on supported builds;
- C2PA network/HTTP default features are disabled in favor of local file I/O and Rust-native crypto;
- Sigstore verification consumes an adjacent explicit bundle and the recorded digest rather than performing arbitrary artifact downloads;
- parsing errors are surfaced as `INVALID`, `UNAVAILABLE` or `NOT_PRESENT`, not swallowed into success.

## Network and privacy threats

- Core features require no account or hosted OriginKeep backend.
- Local files are not uploaded for hashing, comparison, cleanup, restore, move scanning or Trust Lens.
- Freshness checks contact only a validated recorded public HTTP(S) source after an explicit user action.
- OriginKeep does not collect browser cookies or persist authenticated web sessions.
- Remote responses are metadata evidence and are not interpreted as executable code by the lifecycle engine.
- Embedded URL credentials are rejected by the hardened freshness path.
- Browser context can contain sensitive page text/URLs; the candidate cache is intentionally short-lived and only matched context reaches the local database.

See [`PRIVACY.md`](../PRIVACY.md) for the user-facing data-handling policy.

## Cross-platform/browser boundary

Chrome, Edge and Firefox expose the automatic download/native-messaging path used by OriginKeep.

Safari currently does not expose the WebExtensions `downloads` key used by that path. OriginKeep therefore does not ship a misleading automatic Safari download capture implementation. On macOS, local adoption plus `kMDItemWhereFroms` provenance provides the supported bridge.

Platform metadata can disappear when files move through archives, cloud services or filesystems. OriginKeep therefore treats OS provenance as supplemental evidence and keeps SHA-256/File Passport identity independently.

## Release supply-chain threats

GitHub Actions builds desktop/browser artifacts from repository source. Release workflows use the permissions required for draft-release creation and artifact attestation. Third-party actions are pinned to full commit SHAs so the reviewed workflow does not silently follow a moved tag.

Release automation builds from one tag:

- Windows NSIS;
- macOS DMG;
- Linux DEB/AppImage;
- separate Chromium companion ZIP;
- separate Firefox companion ZIP.

The native host is bundled as a Tauri external binary on each desktop target.

Artifact attestation is not platform code signing/notarization and is not a guarantee that a binary is vulnerability-free. Public distribution should configure project-owned signing/notarization credentials and review the draft packages before publication.

The repository must never commit private signing keys, certificate passwords, browser credentials or access tokens.

## Residual risks / 2.0 limitations

- The local archive shares the same machine and usually the same disk as tracked files; it is recoverable cleanup, not disaster recovery.
- Filesystem permissions, endpoint security software or concurrent external changes can interrupt operations.
- Authenticated/expiring remote sources may remain unverifiable without weakening the no-session-replay rule.
- PDF comparison covers extracted text layers only; it does not prove visual/layout equivalence.
- OriginKeep does not provide malware detection or sandbox untrusted documents.
- C2PA/Sigstore/OS provenance are not universally present.
- An embedded Sigstore trust-root snapshot may lag a later ecosystem update until OriginKeep dependencies are refreshed.
- Browser-store publication can assign IDs different from development/release packages; native-host allowlists must be updated/tested for the real published IDs.
- Safari automatic download capture is not supported under the current Safari WebExtensions API surface.
- Unsigned/notarized desktop release candidates can trigger platform reputation/security warnings until project-owned signing is configured.

## Security rule

When OriginKeep cannot prove an identity-sensitive or destructive operation is safe from locally available evidence, it must fail closed and preserve whichever verified copy/evidence still exists.
