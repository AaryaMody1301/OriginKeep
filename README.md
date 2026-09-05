# OriginKeep

**Downloads that remember where they came from.**

OriginKeep is a local-first desktop application and browser companion for preserving download provenance, tracking source freshness, identifying duplicate and superseded files, and keeping version history explainable.

The core relationship is:

```text
local file <-> origin <-> remote state <-> version lineage
```

OriginKeep is intentionally **not** a download accelerator, cloud drive, or AI-first file organizer. Core functionality is designed to work locally without accounts, paid APIs, or a hosted backend.

## Product goals

- Capture browser download provenance: initiating URL, final URL, referrer/source page when supplied, filename, MIME type, size, and timestamps.
- Fingerprint local files so exact duplicates and local modifications are deterministic.
- Group later downloads from the same source into version families.
- Verify whether a public remote source is current, changed, missing, or unverifiable using standard HTTP evidence.
- Compare supported local and remote versions without uploading private files.
- Make cleanup recoverable by retaining source and integrity metadata when a user removes a local copy.

## Planned states

`CURRENT` · `CHANGED` · `DUPLICATE` · `SUPERSEDED` · `LOCAL_MODIFIED` · `SOURCE_MISSING` · `SOURCE_UNKNOWN` · `AUTH_REQUIRED` · `LOCAL_MISSING` · `CHECK_FAILED`

## Architecture

```text
Chrome / Edge extension
        |
        | download metadata
        v
Native Messaging host
        |
        v
OriginKeep desktop (Tauri + React + TypeScript + Rust)
        |
        +-- filesystem + SHA-256
        +-- SQLite metadata store
        +-- provenance/version engine
        +-- conditional HTTP freshness checker
        +-- local PDF/text/CSV comparison engines
```

## Roadmap

### Phase 1 - Provenance foundation

Desktop shell, browser download capture, native-message contract, SQLite provenance schema, SHA-256 fingerprinting, file detail/search surfaces, tests, and CI. **Completed.**

### Phase 2 - Version intelligence

Canonical source identities, exact duplicate detection, deterministic version families, local-modification detection, and version timelines. **Completed.**

### Phase 3 - Living downloads

Conditional HTTP freshness checks, explicit remote-state evidence, remote disappearance/authentication handling, and local PDF/text/CSV comparisons. **In progress.**

### Phase 4 - Safe lifecycle

Recoverable cleanup, source-aware restore, storage review, retention policies, release packaging, migration/recovery testing, and production hardening.

## Current status

Phases 1 and 2 are merged. Phase 3 adds explicit, user-triggered HTTP freshness evidence and bounded local comparison without weakening the local-first privacy boundary.

Remote checks use stored HTTP validators when available, record append-only evidence, and intentionally keep the first baseline-only check as `SOURCE_UNKNOWN` rather than guessing that a source is current. Local comparison supports UTF-8 text, CSV, and PDF text layers; files are not uploaded.

See:

- [`docs/PHASE1.md`](docs/PHASE1.md) for provenance foundation rules.
- [`docs/PHASE2.md`](docs/PHASE2.md) for identity, duplicate, versioning, migration, and acceptance rules.
- [`docs/PHASE3.md`](docs/PHASE3.md) for freshness-state evidence and local comparison rules.

## Privacy boundary

OriginKeep is local-first. The core does not require a user account, cloud database, paid AI API, or hosted backend. Files remain local. Remote freshness checks contact only the recorded HTTP(S) source after an explicit user action; they do not upload the local file.

## License

No open-source license has been selected yet. Until a license is added, normal copyright rules apply.
