# OriginKeep

**Every file remembers where it came from, why you saved it, whether it changed, and how to get it back.**

OriginKeep is a local-first **File Passport** application for preserving file provenance, context, identity, integrity, freshness, version lineage, authenticity evidence and recoverable lifecycle metadata.

OriginKeep 2.0 keeps the v0.1 download engine, but expands the product beyond the Downloads folder:

```text
origin + context + SHA identity + trust evidence + freshness + lineage + recovery
                              |
                              v
                        File Passport
```

Core functionality works without an OriginKeep account, hosted backend, paid API, mandatory cloud sync or file upload.

## File Passport

Each tracked/adopted file can answer:

- **Origin** — where did the file come from?
- **Context** — what page/link was involved, and why did the user save it?
- **Identity** — what immutable SHA-256 identifies these bytes?
- **Integrity** — do the current local bytes still match the recorded fingerprint?
- **Authenticity evidence** — is there C2PA or Sigstore evidence, and what exactly validates?
- **Freshness** — what does the remote HTTP evidence prove right now?
- **Lineage** — is this a version, an exact duplicate, or a new source family?
- **Recovery** — can the file be safely archived/restored without losing unique or modified bytes?

Trust Lens deliberately does **not** collapse these signals into a vague safety score.

## What OriginKeep 2.0 adds

- Captures matched page title, page URL, clicked-link text and bounded nearby context for Chrome/Edge/Firefox downloads.
- Stores optional purpose, note, review/expiry date and retention intent.
- Exports/imports portable `.originkeep.json` passports, verified against the adjacent file's SHA-256.
- Tracks file location history after rename/move recovery.
- Relinks missing files only after exact SHA-256 identity verification.
- Scans an explicitly selected directory to recover moved tracked files.
- Adopts existing local files, including files downloaded before OriginKeep was installed.
- Imports best-effort OS provenance: Windows Zone.Identifier, macOS `kMDItemWhereFroms`, Linux download-uri metadata where available.
- Shows a deterministic Origin Graph of sources, versions and exact duplicates.
- Adds Trust Lens for origin, integrity, remote evidence, OS provenance, C2PA and optional adjacent Sigstore bundles.
- Adds automatic browser capture for Firefox alongside Chrome and Edge.
- Adds Windows, macOS and Linux desktop bundle configurations.
- Packages separate Chromium and Firefox browser companions from the same release tag.

## Browser and platform support

| Platform | Chrome | Edge | Firefox | Safari |
| --- | --- | --- | --- | --- |
| Windows | automatic provenance + context | automatic provenance + context | automatic provenance + context | n/a |
| macOS | automatic provenance + context | automatic provenance + context | automatic provenance + context | local adoption + macOS provenance |
| Linux | automatic provenance + context | supported where Edge is installed | automatic provenance + context | n/a |

Safari is intentionally different. Apple's current Safari Web Extension tooling does not expose the WebExtensions `downloads` API used by the automatic capture path. OriginKeep therefore supports Safari/macOS files through local adoption plus `kMDItemWhereFroms` provenance when available rather than claiming unsupported automatic parity.

See [`docs/BROWSERS.md`](docs/BROWSERS.md).

## Existing provenance/version/lifecycle engine

OriginKeep retains the completed v0.1 capabilities:

- browser download provenance: initiating URL, final URL, referrer, filename, MIME, size and timestamps;
- local SHA-256 fingerprints;
- exact duplicate detection independent of filenames;
- conservative canonical source identities and deterministic version families;
- local modification detection without rewriting the download-time fingerprint;
- explicit HTTP freshness checks using validators and truthful uncertainty/authentication states;
- local text, CSV and PDF text-layer comparison;
- Downloads Review and keep-latest-N cleanup preview;
- SHA-verified recoverable archive;
- collision-safe restore;
- crash/interruption reconciliation through a SQLite lifecycle ledger.

## Evidence states

Remote/version states:

`CURRENT` · `CHANGED` · `DUPLICATE` · `SUPERSEDED` · `SOURCE_MISSING` · `SOURCE_UNKNOWN` · `AUTH_REQUIRED` · `CHECK_FAILED`

Local/lifecycle states:

`PRESENT` · `LOCAL_MODIFIED` · `LOCAL_MISSING` · `ARCHIVING` · `ARCHIVED` · `RESTORING` · `ERROR`

Trust Lens states are signal-specific. For example, C2PA distinguishes `TRUSTED`, `VALID_UNTRUSTED`, `INVALID`, `NOT_PRESENT` and unavailable/missing cases instead of turning them into one product-level score.

## Architecture

```text
Chrome / Edge / Firefox companion     Existing/local file adoption
               |                                |
               | provenance + context           | OS provenance + SHA
               v                                v
                  Bundled Rust native / desktop core
                               |
                               v
                    OriginKeep File Passport
                               |
        +----------------------+----------------------+
        |                      |                      |
        v                      v                      v
 SHA + location history   source/version graph   Trust Lens
        |                      |                C2PA / Sigstore
        +----------------------+----------------------+
                               |
                               v
        remote freshness + local diff + recoverable lifecycle
```

Desktop: **Tauri + React + TypeScript + Rust + SQLite**.

## Portable passports

Export creates:

```text
report.pdf
report.pdf.originkeep.json
```

The portable format deliberately omits absolute local filesystem paths and local location history. Import locates the adjacent file, computes SHA-256 locally and rejects the passport on mismatch before ingesting any provenance metadata.

See [`docs/PASSPORT_SPEC.md`](docs/PASSPORT_SPEC.md).

## Trust Lens

Trust Lens independently reports:

1. browser/source origin evidence;
2. local SHA-256 integrity;
3. latest remote freshness evidence;
4. imported operating-system provenance;
5. C2PA validation when a readable manifest exists;
6. Sigstore verification when an adjacent `<file>.sigstore.json` bundle exists.

A valid C2PA/Sigstore credential proves only what that cryptographic evidence establishes. It is not an antivirus result and does not prove that content is true, safe or appropriate.

## Browser context privacy

Chrome/Edge/Firefox context capture is designed to avoid building a generic browsing-history database:

- context candidates are created only when an HTTP(S) link is activated;
- the extension keeps at most 30 candidates for roughly two minutes;
- session-only browser storage is used when available, with memory-only fallback;
- matched context is consumed;
- unmatched context is not written to OriginKeep's SQLite database;
- no cookie/history permissions are requested.

See [`PRIVACY.md`](PRIVACY.md).

## Security model

OriginKeep treats browser metadata, filesystem paths, portable passports, cryptographic sidecars and remote servers as untrusted inputs.

- Native messages are length-bounded JSON.
- Browser native-host manifests allow only the intended extension IDs/origins.
- Remote checks are explicit user actions and support only public HTTP(S) destinations.
- Redirects are validated hop-by-hop; private/loopback/link-local/reserved targets are rejected and approved DNS results are pinned into the request client.
- OriginKeep never replays browser cookies/login sessions for freshness checks; protected sources remain `AUTH_REQUIRED` when necessary.
- Portable passport import and file relinking require exact SHA-256 matches.
- Local file contents are not uploaded by core functionality.
- Cleanup fails closed if fingerprint evidence does not prove the local bytes are unchanged.
- Restore refuses to overwrite conflicting bytes.

See [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md), [`SECURITY.md`](SECURITY.md), and [`PRIVACY.md`](PRIVACY.md).

## Browser integrations

### Chrome / Edge

Chromium Manifest V3 package:

- `downloads`
- `nativeMessaging`
- `storage`
- HTTP(S) context content script

Stable development/release-package ID:

`mplmkmbnahpggimgfihfgieamonbbobh`

### Firefox

Firefox package uses the same capture logic with a browser-specific manifest and explicit ID:

`originkeep@aaryamody.local`

Firefox Native Messaging uses `allowed_extensions`; Chromium uses `allowed_origins`.

### Registration

- Windows: NSIS registers Chrome, Edge and Firefox native-host manifests for the current user.
- macOS/Linux: use **Register browser integrations** inside OriginKeep. A development shell helper is also available at `scripts/install-native-host-unix.sh`.

## Cross-platform desktop packaging

Tauri configurations are included for:

- Windows NSIS: `npm run bundle:windows`
- macOS app/DMG: `npm run bundle:macos`
- Linux DEB/AppImage: `npm run bundle:linux`

All platforms bundle the Rust native host as a Tauri external binary.

The tag-triggered release workflow creates a draft release and builds/uploads:

- Windows NSIS installer;
- macOS DMG;
- Linux DEB and AppImage;
- Chromium companion ZIP;
- Firefox companion ZIP;
- GitHub artifact attestations for release assets.

Platform signing/notarization remains credential-dependent and is an external release requirement rather than embedded secret material.

## Development

Requirements:

- Node.js 22+
- Rust stable
- Tauri 2 prerequisites for the target desktop platform

Typical commands:

```bash
npm ci
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked
npm run tauri dev
```

Browser package staging:

```bash
npm run stage:companions
```

## Product history

### v0.1 foundation — completed

- Phase 1: provenance foundation
- Phase 2: deterministic version intelligence
- Phase 3: living-download freshness and local comparisons
- Phase 4: safe recoverable lifecycle
- release-candidate hardening: bundled native host, network boundary, reproducible/pinned builds, Windows installer smoke CI

### OriginKeep 2.0 — File Passport expansion

Universal file adoption, context, portable passports, move/rename identity, OS provenance, Trust Lens, Origin Graph, Firefox support and cross-platform desktop packaging.

See [`docs/ORIGINKEEP2.md`](docs/ORIGINKEEP2.md).

## Documentation

- [`docs/ORIGINKEEP2.md`](docs/ORIGINKEEP2.md) — File Passport product contract and compatibility matrix.
- [`docs/PASSPORT_SPEC.md`](docs/PASSPORT_SPEC.md) — portable passport format and SHA invariant.
- [`docs/BROWSERS.md`](docs/BROWSERS.md) — Chrome/Edge/Firefox/Safari behavior and privacy boundary.
- [`docs/PHASE1.md`](docs/PHASE1.md) — provenance foundation.
- [`docs/PHASE2.md`](docs/PHASE2.md) — source identity, duplicates and versioning.
- [`docs/PHASE3.md`](docs/PHASE3.md) — remote evidence and local comparisons.
- [`docs/PHASE4.md`](docs/PHASE4.md) — lifecycle invariants, retention and recovery.
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — security boundaries and residual risks.
- [`docs/RELEASE.md`](docs/RELEASE.md) — release acceptance checklist.
- [`PRIVACY.md`](PRIVACY.md) — local-first data handling.
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting.

## Non-goals

OriginKeep remains intentionally **not**:

- an antivirus/malware verdict engine;
- an AI chatbot;
- a cloud drive or mandatory sync service;
- a generic download accelerator/media grabber;
- a filename-based auto organizer;
- an autonomous hidden deletion tool;
- a credential/session collector.

## License

No open-source license has been selected. Until the repository owner chooses and adds a license, normal copyright rules apply.
