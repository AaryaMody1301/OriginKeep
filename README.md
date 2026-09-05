# OriginKeep

**Every file remembers where it came from, why you saved it, whether it changed, and how to get it back.**

OriginKeep is a local-first **File Passport** system for downloaded and existing files. It combines browser provenance, deterministic content identity, source freshness, version lineage, authenticity evidence and recoverable cleanup without requiring an account, hosted backend or file upload.

```text
origin + context + exact bytes + trust evidence + freshness + lineage + recovery
                                |
                                v
                         ORIGINKEEP PASSPORT
```

OriginKeep is intentionally not an antivirus, cloud drive, download accelerator, AI chatbot or automatic destructive file organizer.

## What makes OriginKeep different

A normal download remembers mostly its filename and folder. An OriginKeep Passport can answer:

- **Origin** — where did these bytes come from?
- **Context** — what page/link was I looking at and why did I save it?
- **Identity** — is this still the same exact file after a move, rename or copy?
- **Integrity** — did my local bytes change?
- **Freshness** — is there evidence that the remote source changed?
- **Authenticity** — what can Windows, C2PA or Sigstore independently verify?
- **Lineage** — which deterministic version or exact duplicate is this?
- **Recovery** — can I archive it safely and restore it without overwriting other data?

There is no opaque AI or “trust score.” Each state is backed by inspectable evidence.

## Core capabilities

### Universal File Passport

- persistent origin/context/identity/freshness/lineage/recovery view;
- optional user purpose, note and review/expiry metadata;
- portable `<file>.originkeep.json` export/import linked to exact SHA-256;
- open documented passport format in [`docs/PASSPORT_SPEC.md`](docs/PASSPORT_SPEC.md).

### Download context

Chrome/Edge/Firefox can capture the download URL, final URL, referrer, filename, MIME, size and timestamps.

Enhanced context is **opt-in** from the companion popup. When enabled it can additionally retain:

- page title and URL;
- clicked link/button text;
- bounded nearby page text.

The extra HTTP/HTTPS host permission is requested only after the user explicitly enables enhanced context.

### Content identity after moves and copies

SHA-256 is the identity anchor. OriginKeep records multiple verified locations for identical bytes and can reconnect a moved file only after an exact hash match. Filename similarity never establishes identity.

### Existing-file adoption

Files that predate OriginKeep can be adopted and hashed.

- Windows: imports `Zone.Identifier` / Mark-of-the-Web evidence when available.
- macOS: imports `kMDItemWhereFroms` when available.
- Linux: keeps source unknown unless evidence or a user-provided source exists.
- Safari fallback context can be associated with a later macOS adoption without claiming automatic Downloads API parity.

### Trust Lens

Trust Lens reports separate evidence channels:

- local SHA-256 integrity — built in;
- Windows origin metadata — Windows;
- Authenticode — Windows PowerShell verification;
- C2PA Content Credentials — optional official `c2patool` integration;
- Sigstore — optional `cosign verify-blob` integration with an explicit expected identity and OIDC issuer.

Unavailable tools produce explicit unavailable/policy-required states. Absence of a signature is not treated as malware, and presence of metadata is not treated as truth.

### Living downloads

- canonical HTTP(S) source identities;
- deterministic version families and exact duplicates;
- conditional ETag / Last-Modified checks;
- explicit `CURRENT`, `CHANGED`, `AUTH_REQUIRED`, `SOURCE_MISSING`, `SOURCE_UNKNOWN`, `CHECK_FAILED` states;
- hardened redirect/DNS/private-network boundary;
- local text, CSV and PDF text-layer comparison.

### Recoverable lifecycle

- Downloads Review with explicit keep-latest-N policy;
- duplicate/superseded cleanup candidates;
- archive only after immutable SHA-256 re-verification;
- collision-safe restore;
- crash/interruption reconciliation;
- storage analytics and lifecycle ledger.

### Origin Graph

OriginKeep derives an evidence graph rather than an AI graph:

```text
SOURCE --PRODUCED--> FILE --HAS_CONTENT--> SHA-256 --LOCATED_AT--> PATH
                    |
                    +--NEXT_VERSION--> FILE
                    +--SAME_CONTENT--> SHA-256
```

## Browser support

| Browser | Automatic provenance | Native host | Enhanced context |
| --- | --- | --- | --- |
| Chrome | Yes | Yes | Optional |
| Edge | Yes | Yes | Optional |
| Chromium | Yes | Yes | Optional |
| Firefox | Yes | Yes | Optional |
| Brave / Vivaldi | Chromium API dependent | Linux/macOS registration included | Optional |
| Safari | Adoption-based fallback | Safari containing-app bridge | Optional fallback context |

Chromium and Firefox are packaged separately because Firefox MV3 uses its background-script model and explicit Gecko extension ID.

See [`docs/ORIGINKEEP_2.md`](docs/ORIGINKEEP_2.md) and [`docs/SAFARI.md`](docs/SAFARI.md).

## Desktop support

| Platform | Packages | Browser integration |
| --- | --- | --- |
| Windows | NSIS | Installer registers Chrome, Edge and Firefox |
| Linux | AppImage + Debian package | App installs stable per-user native-host manifests |
| macOS | `.app` + DMG | App installs stable per-user native-host manifests |

macOS CI uses ad-hoc signing to verify the build. Trusted public macOS distribution still requires Apple signing/notarization.

## Architecture

```text
Chrome / Edge / Firefox
        | downloads + optional context
        v
Native Messaging host (Rust)
        |
        +------------------------------+
        |                              |
        v                              v
OriginKeep desktop              Safari fallback context
Tauri + React + Rust            via containing-app bridge
        |
        +-- SQLite Passport/context/trust/location ledger
        +-- SHA-256 content identity
        +-- deterministic version/duplicate engine
        +-- hardened conditional HTTP freshness checker
        +-- local PDF/text/CSV comparison
        +-- Trust Lens evidence adapters
        +-- verified archive + collision-safe restore
        +-- portable File Passport import/export
```

## Security and privacy

OriginKeep treats browser metadata, local paths, portable passports and remote servers as untrusted input.

- Native messages are length-bounded JSON.
- Browser native hosts allow only explicit extension IDs/origins.
- Enhanced browsing context is optional and stored locally.
- Remote freshness checks are explicit and public HTTP(S)-only.
- Every redirect destination is revalidated and private/loopback/link-local/reserved destinations are blocked.
- Browser cookies/authenticated session credentials are not replayed.
- Local file contents are never uploaded by core functionality.
- Cleanup fails closed if SHA-256 evidence no longer matches.
- Restore refuses to overwrite conflicting bytes.
- Portable passport import re-hashes the adjacent file before accepting metadata.

See [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md), [`PRIVACY.md`](PRIVACY.md), and [`SECURITY.md`](SECURITY.md).

## Development

Requirements:

- Node.js 22+
- Rust stable
- Tauri 2 platform prerequisites for the platform being built

```bash
npm ci
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked
npm run tauri dev
```

Browser packages:

```bash
npm run prepare:browsers
```

Desktop bundles:

```bash
npm run bundle:windows
npm run bundle:linux
npm run bundle:macos
```

Safari Xcode project generation must be run on macOS with Xcode:

```bash
./scripts/prepare-safari-project.sh
```

## Implementation history

- **Phase 1 — Provenance foundation:** completed.
- **Phase 2 — Version intelligence:** completed.
- **Phase 3 — Living downloads:** completed.
- **Phase 4 — Safe lifecycle:** completed.
- **v0.1 release hardening:** completed.
- **OriginKeep 2.0 — Universal File Passport:** portable passports, context, multi-location identity, Trust Lens, Origin Graph, Firefox, macOS/Linux packaging, Safari fallback and existing-file adoption.

## Documentation

- [`docs/PASSPORT_SPEC.md`](docs/PASSPORT_SPEC.md) — portable passport contract.
- [`docs/ORIGINKEEP_2.md`](docs/ORIGINKEEP_2.md) — 2.0 capabilities and platform matrix.
- [`docs/SAFARI.md`](docs/SAFARI.md) — Safari packaging/fallback model.
- [`docs/PHASE1.md`](docs/PHASE1.md) through [`docs/PHASE4.md`](docs/PHASE4.md) — original implementation phases.
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) — trust boundaries and residual risks.
- [`docs/RELEASE.md`](docs/RELEASE.md) — release validation.
- [`PRIVACY.md`](PRIVACY.md) — local-first data handling.
- [`SECURITY.md`](SECURITY.md) — vulnerability reporting.

## Deliberate non-goals

Scheduled/background source checks, cloud sync, cookie/session replay, antivirus verdicts, AI cleanup decisions, automatic destructive deletion and a download-speed engine remain outside the core product.

## License

No open-source license has been selected. Until the repository owner chooses and adds a license, normal copyright rules apply.
