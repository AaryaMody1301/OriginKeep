# OriginKeep

**Downloads that remember where they came from.**

OriginKeep is a local-first desktop application and browser companion for preserving download provenance, tracking source freshness, identifying duplicate and superseded files, and keeping version history explainable.

The core relationship is:

```text
local file <-> origin <-> remote state <-> version lineage <-> recoverable lifecycle
```

OriginKeep is intentionally **not** a download accelerator, cloud drive, or AI-first file organizer. Core functionality is designed to work locally without accounts, paid APIs, or a hosted backend.

## Product goals

- Capture browser download provenance: initiating URL, final URL, referrer/source page when supplied, filename, MIME type, size, and timestamps.
- Fingerprint local files so exact duplicates and local modifications are deterministic.
- Group later downloads from the same source into version families.
- Verify whether a public remote source is current, changed, missing, or unverifiable using standard HTTP evidence.
- Compare supported local versions without uploading private files.
- Make cleanup recoverable by retaining source, version and integrity metadata after a local copy moves into the OriginKeep archive.
- Fail closed when local evidence cannot prove that archive or restore is safe.

## Evidence and lifecycle states

Remote/version states:

`CURRENT` · `CHANGED` · `DUPLICATE` · `SUPERSEDED` · `SOURCE_MISSING` · `SOURCE_UNKNOWN` · `AUTH_REQUIRED` · `CHECK_FAILED`

Local/lifecycle states:

`PRESENT` · `LOCAL_MODIFIED` · `LOCAL_MISSING` · `ARCHIVING` · `ARCHIVED` · `RESTORING` · `ERROR`

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
        +-- SQLite metadata + lifecycle ledger
        +-- provenance/version engine
        +-- conditional HTTP freshness checker
        +-- local PDF/text/CSV comparison engines
        +-- verified local archive + collision-safe restore
```

## Roadmap

### Phase 1 - Provenance foundation

Desktop shell, browser download capture, native-message contract, SQLite provenance schema, SHA-256 fingerprinting, file detail/search surfaces, tests, and CI. **Completed.**

### Phase 2 - Version intelligence

Canonical source identities, exact duplicate detection, deterministic version families, local-modification detection, and version timelines. **Completed.**

### Phase 3 - Living downloads

Conditional HTTP freshness checks, explicit remote-state evidence, remote disappearance/authentication handling, and local PDF/text/CSV comparisons. **Completed.**

### Phase 4 - Safe lifecycle

Recoverable cleanup, storage review, retention-policy previews, collision-safe restore, migration/recovery testing, NSIS Windows packaging, artifact attestations, threat modeling, and production hardening. **In progress on the Phase 4 branch.**

## Phase 4 Downloads Review

The final phase adds a deterministic `Downloads Review` instead of an opaque cleanup score. The review reports tracked, duplicate, superseded, archived and policy-selected byte totals and lets the user preview a keep-latest-N policy.

A recommendation never deletes a file automatically. `Archive safely` is explicit and requires the current local SHA-256 to match the immutable download fingerprint. OriginKeep copies the file into its local application-data archive, flushes and re-hashes the copy, and only then removes the original path. Restore verifies the archive and refuses to overwrite different bytes already present at the original location.

Interrupted archive/restore states are written to SQLite before filesystem mutation and reconciled on the next launch from whichever copy can still be verified.

## Release engineering

The Phase 4 branch enables Tauri's NSIS Windows bundle target and includes a tag/manual GitHub Actions release workflow. The workflow generates platform icons, builds the Windows installer, creates a **draft** GitHub release and generates an artifact attestation for the installer.

Windows code-signing credentials are intentionally not stored in the repository. A public installer should remain draft until project-owned signing credentials are configured and the resulting package is reviewed. Artifact attestation establishes build provenance; it is not a substitute for Authenticode signing or a security audit.

## Current status

Phases 1–3 are merged. Phase 4 is the final planned implementation phase and is being validated through the same strict frontend build, Rust formatting, Clippy and test gates used by earlier phases.

See:

- [`docs/PHASE1.md`](docs/PHASE1.md) for provenance foundation rules.
- [`docs/PHASE2.md`](docs/PHASE2.md) for identity, duplicate, versioning, migration, and acceptance rules.
- [`docs/PHASE3.md`](docs/PHASE3.md) for freshness-state evidence and local comparison rules.
- [`docs/PHASE4.md`](docs/PHASE4.md) for lifecycle invariants, recovery rules, retention policy and release acceptance.
- [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) for trust boundaries, destructive-operation defenses and residual risks.

## Privacy boundary

OriginKeep is local-first. The core does not require a user account, cloud database, paid AI API, or hosted backend. Files remain local. Remote freshness checks contact only the recorded HTTP(S) source after an explicit user action; they do not upload the local file. Recoverable archive copies also remain in local OriginKeep application data.

## License

No open-source license has been selected yet. Until a license is added, normal copyright rules apply.
