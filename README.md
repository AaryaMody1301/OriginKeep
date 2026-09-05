# OriginKeep

**Every file remembers where it came from, why you saved it, whether it changed, and how to get it back.**

OriginKeep is a local-first desktop application and browser companion for persistent file provenance. It combines download origin, save context, SHA-256 content identity, version lineage, remote freshness evidence, authenticity signals and recoverable lifecycle actions in a single **File Passport**.

The core relationship is:

```text
file bytes <-> origin/context <-> content identity <-> trust evidence
           <-> remote state <-> version lineage <-> user intent <-> recovery
```

OriginKeep is intentionally **not** a download accelerator, cloud drive, malware scanner, or AI-first file organizer. Core functionality works without an account, hosted backend, paid API, file upload, or LLM.

## Universal File Passport

Each tracked/adopted file can answer:

- **Origin** — where did these bytes come from?
- **Context** — what page/link was involved and why did I save it?
- **Identity** — is this the same content after rename or move?
- **Integrity** — do the current bytes still match the immutable download/adoption SHA-256?
- **Authenticity evidence** — what do platform signatures, C2PA or Sigstore actually prove?
- **Freshness** — does the recorded public source provide deterministic evidence of change?
- **Lineage** — which exact version/duplicate relationships exist?
- **Intent** — is this reference material, temporary, never-to-archive, or review-on-change?
- **Recovery** — can it be archived/restored without destroying unique local data?

Portable passports are adjacent JSON sidecars such as:

```text
report.pdf
report.pdf.originkeep.json
```

Import verifies the selected file against the passport SHA-256 before reconnecting metadata. Absolute local paths are deliberately excluded from the portable format.

See [`docs/PASSPORT_SPEC.md`](docs/PASSPORT_SPEC.md).

## What OriginKeep does

- Captures browser download provenance: initiating URL, final URL, referrer when supplied, filename, MIME type, size and timestamps.
- Optionally captures page title, source page, clicked link text and bounded nearby context after the user grants richer browser access.
- Lets users **adopt any existing local file** into OriginKeep even if it predates the browser companion; unknown origin remains explicitly unknown unless supplied by the user.
- Computes local SHA-256 fingerprints so exact duplicates, local modification and move/rename identity are deterministic.
- Tracks known file locations and can relink a moved file by exact SHA-256, never filename guessing.
- Exports/imports portable File Passports bound to content fingerprints.
- Groups primary downloads from the same canonical source into version families and exposes a site → source → file Origin Graph.
- Verifies public remote freshness using HTTP validators without claiming certainty when evidence is weak.
- Preserves `AUTH_REQUIRED` rather than replaying browser credentials/cookies for protected sources.
- Compares supported local text, CSV and PDF text-layer versions without uploading files.
- Provides a Trust Lens with local integrity, recorded/platform origin, platform publisher-signature evidence, optional C2PA verification and optional Sigstore verification.
- Reviews duplicate/superseded/expired storage with explicit user intent and keep-latest-N rules.
- Archives cleanup candidates only after re-verifying the immutable fingerprint.
- Restores archived bytes without overwriting different data at the original path.
- Recovers interrupted archive/restore operations from an explicit SQLite lifecycle ledger.

## Evidence states

Remote/version states:

`CURRENT` · `CHANGED` · `DUPLICATE` · `SUPERSEDED` · `SOURCE_MISSING` · `SOURCE_UNKNOWN` · `AUTH_REQUIRED` · `CHECK_FAILED`

Local/lifecycle states:

`PRESENT` · `LOCAL_MODIFIED` · `LOCAL_MISSING` · `ARCHIVING` · `ARCHIVED` · `RESTORING` · `ERROR`

Passport lifecycle intents:

`MANUAL` · `REVIEW_WHEN_NEWER` · `ARCHIVE_WHEN_SUPERSEDED` · `ARCHIVE_WHEN_EXPIRED` · `NEVER_ARCHIVE`

Intent changes **recommendations only**. OriginKeep does not silently delete or archive user files.

## Architecture

```text
Chrome / Edge / Chromium / Firefox companion
                 |
                 | download metadata + optional page context
                 v
Bundled Native Messaging host (Rust)
                 |
                 v
OriginKeep desktop (Tauri + React + TypeScript + Rust)
                 |
                 +-- Windows / macOS / Linux
                 +-- SQLite provenance + passport + lifecycle data
                 +-- filesystem + SHA-256 content identity
                 +-- portable .originkeep.json passports
                 +-- deterministic version/duplicate/origin graph engine
                 +-- hardened conditional HTTP freshness checker
                 +-- local PDF/text/CSV comparison engines
                 +-- Trust Lens (platform + optional C2PA/Sigstore)
                 +-- verified archive + collision-safe restore
```

## Browser support

### Chromium family

The Chromium Manifest V3 companion uses required permissions for download provenance/native messaging/local extension storage plus the scripting API. Rich page context is **not** granted at install time: HTTP(S) host access and `tabs` are optional and requested only when the user clicks the companion action to enable richer context.

The repository release package uses a public manifest key so unpacked/release-package installs have deterministic extension ID:

`mplmkmbnahpggimgfihfgieamonbbobh`

### Firefox

Firefox has a separate Manifest V3 package with explicit add-on ID:

`originkeep@aaryamody1301.github.io`

OriginKeep uses Firefox's `allowed_extensions` Native Messaging manifest rather than Chromium's `allowed_origins`. Windows NSIS registers Firefox automatically; macOS/Linux registrations are created per-user by the desktop bridge.

### Safari

Safari Web Extensions can share WebExtension code and communicate with a containing macOS app, but Apple's current Safari extension tooling does **not** support the `downloads` manifest capability OriginKeep relies on for automatic provenance capture. OriginKeep therefore does not claim automatic Safari download-event parity.

The macOS desktop app, local adoption, portable passports and non-Safari-specific features remain available. See [`docs/SAFARI.md`](docs/SAFARI.md).

## Desktop platforms

OriginKeep's Tauri desktop configuration and CI produce platform-appropriate bundles:

- Windows x64 — NSIS installer with bundled Native Messaging host.
- macOS — unsigned DMG release-candidate build with bundled host and per-user Chrome/Chromium/Edge/Firefox bridge registration at runtime.
- Linux — AppImage and DEB release-candidate builds with bundled host and per-user Chrome/Chromium/Edge/Firefox registration at runtime.

Unsigned macOS/Windows builds can trigger platform reputation/security prompts. Stable public distribution should use appropriate platform signing/notarization credentials.

## Trust Lens

OriginKeep never collapses authenticity into an opaque score.

- SHA-256 proves whether local bytes match OriginKeep's recorded baseline.
- Windows Mark of the Web or macOS `kMDItemWhereFroms` are reported when available; absence is not treated as proof of local origin.
- Windows Authenticode/macOS code-signing checks are platform evidence, not an antivirus verdict.
- If `c2patool` is installed, OriginKeep asks it to parse/validate C2PA manifests. Cryptographic validation is presented separately from issuer trust.
- If a `.sigstore.json` bundle is adjacent to an artifact, OriginKeep reports it. Explicit Sigstore verification requires `cosign`, an expected certificate identity and an expected OIDC issuer.

No C2PA/Sigstore state is claimed as verified merely because marker bytes or a bundle filename exists.

## Security model

OriginKeep treats browser metadata, filesystem paths, portable passport JSON and remote servers as untrusted inputs.

- Native messages are length-bounded JSON and host allowlists are browser-specific.
- Rich browser page context is opt-in and bounded before native ingestion.
- Remote checks are explicit user actions and support only public HTTP(S) destinations.
- Release builds disable automatic redirects, validate every redirect hop, reject private/loopback/link-local/reserved targets, and pin validated DNS results into the request client.
- Local file contents are never uploaded by core functionality.
- Portable passport import and moved-file relinking require exact SHA-256 equality.
- Cleanup fails closed if fingerprint evidence does not prove local bytes are unchanged.
- Restore refuses to overwrite a conflicting file.
- Trust verification invokes local verifier tools with argument arrays rather than shell-concatenating file paths.

See [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md), [`SECURITY.md`](SECURITY.md), and [`PRIVACY.md`](PRIVACY.md).

## Development

Requirements:

- Node.js 22+
- Rust stable
- Tauri 2 prerequisites for the platform being built

Typical commands:

```bash
npm ci
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked
npm run tauri dev
```

Platform bundles:

```bash
npm run bundle:windows
npm run bundle:macos
npm run bundle:linux
```

The release build prepares the target-triple-specific `originkeep-native-host` automatically before Tauri bundles the app.

## Roadmap status

### Phase 1 — Provenance foundation — Completed

Browser capture, Native Messaging, SQLite provenance, SHA-256, search and CI.

### Phase 2 — Version intelligence — Completed

Canonical source identities, exact duplicates, deterministic version families, local modification detection and timelines.

### Phase 3 — Living downloads — Completed

Conditional remote freshness evidence, source disappearance/authentication states, local PDF/text/CSV comparisons.

### Phase 4 — Safe lifecycle — Completed

Downloads Review, retention preview, recoverable archive/restore, crash reconciliation, storage metrics and Windows packaging.

### Release-candidate hardening — Completed

Bundled native-host installation, private-network/redirect protection, frozen dependency builds, pinned CI/release actions, privacy/security docs, installer CI and release checklist.

### OriginKeep 2.0 — Universal File Passport — In development

Portable passports, optional save context, existing-file adoption, content-based move/rename relinking, Origin Graph, intent/expiry policies, evidence-based Trust Lens, Firefox support and Windows/macOS/Linux packaging.

## Documentation

- [`docs/PASSPORT_SPEC.md`](docs/PASSPORT_SPEC.md) — portable File Passport format and integrity rules.
- [`docs/ORIGINKEEP_2.md`](docs/ORIGINKEEP_2.md) — OriginKeep 2.0 acceptance scope.
- [`docs/SAFARI.md`](docs/SAFARI.md) — Safari capability boundary and macOS alternatives.
- [`docs/PHASE1.md`](docs/PHASE1.md) — provenance foundation.
- [`docs/PHASE2.md`](docs/PHASE2.md) — source identity, duplicates and versioning.
- [`docs/PHASE3.md`](docs/PHASE3.md) — remote evidence and local comparison.
- [`docs/PHASE4.md`](docs/PHASE4.md) — lifecycle invariants, retention and recovery.
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — trust boundaries and residual risks.
- [`docs/RELEASE.md`](docs/RELEASE.md) — clean-machine release checklist.
- [`PRIVACY.md`](PRIVACY.md) — local-first data handling.
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting and supported security boundaries.

## Deliberate non-goals

Cloud sync, credential/cookie replay, full download-manager acceleration, malware scanning, silent destructive cleanup and AI-driven cleanup decisions remain outside the core design.

## License

No open-source license has been selected. Until the repository owner chooses and adds a license, normal copyright rules apply.
