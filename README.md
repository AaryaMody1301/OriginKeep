# OriginKeep

**Downloads that remember where they came from.**

OriginKeep is a local-first Windows desktop application and Chromium browser companion for preserving download provenance, tracking source freshness, identifying duplicate and superseded files, comparing versions, and making cleanup recoverable.

The core relationship is:

```text
local file <-> origin <-> remote state <-> version lineage <-> recoverable lifecycle
```

OriginKeep is intentionally **not** a download accelerator, cloud drive, malware scanner, or AI-first file organizer. Core functionality works without an account, hosted backend, paid API, or file upload.

## What OriginKeep does

- Captures browser download provenance: initiating URL, final URL, referrer when supplied, filename, MIME type, size and timestamps.
- Computes local SHA-256 fingerprints so exact duplicates and local modification are deterministic.
- Groups primary downloads from the same canonical source into version families.
- Verifies public remote freshness using HTTP validators without claiming certainty when evidence is weak.
- Compares supported local text, CSV and PDF text-layer versions without uploading files.
- Reviews duplicate/superseded storage with explicit keep-latest-N rules.
- Archives cleanup candidates only after re-verifying the immutable download fingerprint.
- Restores archived bytes without overwriting different data at the original path.
- Recovers interrupted archive/restore operations from an explicit SQLite lifecycle ledger.

## Evidence states

Remote/version states:

`CURRENT` · `CHANGED` · `DUPLICATE` · `SUPERSEDED` · `SOURCE_MISSING` · `SOURCE_UNKNOWN` · `AUTH_REQUIRED` · `CHECK_FAILED`

Local/lifecycle states:

`PRESENT` · `LOCAL_MODIFIED` · `LOCAL_MISSING` · `ARCHIVING` · `ARCHIVED` · `RESTORING` · `ERROR`

## Architecture

```text
Chrome / Edge companion
        |
        | browser download metadata
        v
Bundled Native Messaging host (Rust)
        |
        v
OriginKeep desktop (Tauri + React + TypeScript + Rust)
        |
        +-- filesystem + SHA-256
        +-- SQLite provenance + lifecycle ledger
        +-- deterministic version/duplicate engine
        +-- hardened conditional HTTP freshness checker
        +-- local PDF/text/CSV comparison engines
        +-- verified archive + collision-safe restore
```

## Security model

OriginKeep treats browser metadata, filesystem paths and remote servers as untrusted inputs.

- Native messages are length-bounded JSON and the host is allowlisted to a specific extension origin.
- Remote checks are explicit user actions and support only public HTTP(S) destinations.
- Release builds disable automatic redirects, validate every redirect hop, reject private/loopback/link-local/reserved targets, and pin validated DNS results into the request client.
- Local file contents are never uploaded by core functionality.
- Cleanup fails closed if SHA-256 evidence does not prove the local bytes are unchanged.
- Restore refuses to overwrite a conflicting file.

See [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md), [`SECURITY.md`](SECURITY.md), and [`PRIVACY.md`](PRIVACY.md).

## Browser companion

The Chromium Manifest V3 companion requests only:

- `downloads`
- `nativeMessaging`

The repository release package uses a public manifest key so unpacked/release-package installs have deterministic extension ID:

`mplmkmbnahpggimgfihfgieamonbbobh`

The Windows installer registers the bundled native host for that exact origin in both Edge and Chrome. If a future browser store assigns a different ID, the `allowed_origins` list must be updated before publishing that store package.

## Windows release packaging

Tauri's NSIS installer contains both:

- `originkeep.exe`
- `originkeep-native-host.exe`

The native host is built as a target-triple-specific Tauri external binary. NSIS install/uninstall hooks create and remove the current-user native-messaging registration.

The tag-triggered GitHub Actions release workflow builds the Windows installer, packages the browser companion ZIP, creates a **draft** release, and generates GitHub artifact provenance for the installer.

Windows Authenticode signing remains credential-dependent. An unsigned release candidate can trigger Windows reputation warnings; artifact attestation proves build provenance but is not code signing or a security audit.

## Development

Requirements:

- Node.js 22+
- Rust stable
- current Tauri 2 Windows prerequisites for desktop development

Typical commands:

```bash
npm ci
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked
npm run tauri dev
```

Windows installer build:

```bash
npm run bundle:windows
```

The release build prepares `originkeep-native-host` automatically before Tauri bundles the NSIS installer.

## Roadmap status

### Phase 1 — Provenance foundation

Browser download capture, Native Messaging, SQLite provenance, SHA-256, search and CI. **Completed.**

### Phase 2 — Version intelligence

Canonical source identities, exact duplicates, deterministic version families, local modification detection and timelines. **Completed.**

### Phase 3 — Living downloads

Conditional remote freshness evidence, source disappearance/authentication states, local PDF/text/CSV comparisons. **Completed.**

### Phase 4 — Safe lifecycle

Downloads Review, retention preview, recoverable archive/restore, crash reconciliation, storage metrics, threat model and NSIS packaging. **Completed.**

### Release candidate hardening

Bundled native-host installation, deterministic companion packaging, private-network/redirect protection, frozen dependency builds, pinned CI/release actions, privacy/security documentation, Windows bundle CI and release checklist. **Release gate.**

## Documentation

- [`docs/PHASE1.md`](docs/PHASE1.md) — provenance foundation and development smoke test.
- [`docs/PHASE2.md`](docs/PHASE2.md) — source identity, duplicates, versioning and migration.
- [`docs/PHASE3.md`](docs/PHASE3.md) — remote evidence and local comparison rules.
- [`docs/PHASE4.md`](docs/PHASE4.md) — lifecycle invariants, retention and recovery.
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — trust boundaries and residual risks.
- [`docs/RELEASE.md`](docs/RELEASE.md) — clean Windows release-candidate checklist.
- [`PRIVACY.md`](PRIVACY.md) — local-first data handling.
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting and supported security boundaries.

## Deferred after v0.1

Scheduled freshness checks, notifications, authenticated source sessions, cloud sync, automatic re-download, bulk destructive cleanup and AI-driven cleanup decisions are deliberately outside the v0.1 release scope.

## License

No open-source license has been selected. Until the repository owner chooses and adds a license, normal copyright rules apply.
