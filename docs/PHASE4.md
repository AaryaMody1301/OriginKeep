# Phase 4 — Safe Download Lifecycle

Phase 4 turns OriginKeep's provenance and version evidence into a reversible local storage workflow. It does **not** turn the application into an automatic cleaner: retention rules produce recommendations, while archive and restore remain explicit user actions.

## Goals

Phase 4 answers four practical questions:

1. Which tracked files are deterministic cleanup candidates?
2. Can a file leave its original Downloads path without losing its provenance and integrity evidence?
3. Can that file be restored without overwriting unrelated or locally changed bytes?
4. Can the desktop application be packaged and released with reproducible CI evidence?

## Lifecycle state machine

```text
ACTIVE
  |
  | explicit Archive safely action
  v
ARCHIVING
  |
  | verified copy + original removal + metadata commit
  v
ARCHIVED
  |
  | explicit Restore action
  v
RESTORING
  |
  | verified restore + archive removal + metadata commit
  v
ACTIVE
```

`ERROR` is reserved for an interrupted operation that startup reconciliation cannot prove safe. `ARCHIVING` and `RESTORING` are persisted before filesystem mutation so the next launch can reconcile a crash or partial operation.

## Safety invariants

OriginKeep must satisfy all of these before removing an original tracked file:

- the record has a stored SHA-256 fingerprint;
- the original path currently resolves to a local file;
- the current local SHA-256 still equals the download fingerprint;
- a copy is written to the OriginKeep application-data archive;
- the copied bytes are flushed and re-hashed;
- the archive SHA-256 equals the stored fingerprint;
- only then may the original path be removed.

A local hash mismatch changes the record to `LOCAL_MODIFIED` and blocks archival. OriginKeep never treats a filename, file size, remote ETag, or source URL as a substitute for local integrity verification.

## Restore invariants

Restore uses the stored original path and recorded SHA-256.

- The archive copy must exist and match the stored SHA-256.
- If the original path does not exist, OriginKeep recreates its parent directory and copies the verified archive bytes back.
- If the original path already contains the same bytes, restore may finish without overwriting them.
- If the original path contains different bytes, restore fails instead of overwriting the file.
- The archive copy is removed only after the restored bytes have been verified.

## Retention policy

The `Downloads Review` is deterministic and preview-only.

Users choose:

- how many newest primary versions to keep per source family; and
- whether exact SHA-256 duplicates should be included as candidates.

A record can become an `ARCHIVE_CANDIDATE` only when it is a verified local file with a stored hash and either:

- it is an exact duplicate and duplicate review is enabled; or
- it is a superseded primary version outside the configured keep-latest-N window.

OriginKeep protects:

- the newest retained primary versions;
- `LOCAL_MODIFIED` files;
- files without a stored hash;
- already-missing local paths; and
- any record that does not meet an explicit cleanup rule.

The policy does not execute archival in bulk. Each archive action remains explicit in Phase 4.

## Storage review metrics

The review reports deterministic byte totals for:

- all tracked records;
- currently present tracked files;
- recoverably archived files;
- policy-selected reclaimable files;
- exact duplicate files;
- superseded files; and
- present files protected from cleanup.

These values come from recorded metadata and lifecycle state. They are not estimates produced by AI.

## Local archive layout

Archived bytes stay in the OriginKeep application-data directory beside the SQLite database, under an `archive/` directory. Archive filenames contain the download record ID and a short prefix of the recorded SHA-256, plus a sanitized display filename.

Metadata remains in SQLite after archival, including provenance, version lineage, remote-check evidence, original path and the full SHA-256 fingerprint.

## Recovery and migration

Phase 4 adds the `lifecycle_entries` table and advances `PRAGMA user_version` to `4`.

Startup reconciliation inspects persisted `ARCHIVING` and `RESTORING` operations. It compares whichever original/archive copies remain against the recorded SHA-256 and chooses only an evidence-backed recovery:

- finalize an archive when only a valid archive copy remains;
- roll back to active when the original remains valid;
- finalize a restore when the original is valid;
- keep the archive when restore did not complete; or
- mark the record `ERROR` / `LOCAL_MISSING` when neither side can be verified.

The Downloads Review also exposes SQLite `quick_check` plus foreign-key-check status as database-health evidence.

## Windows packaging

`src-tauri/tauri.conf.json` enables the NSIS bundle target. `npm run icons` derives Tauri's platform icon set from the committed source PNG, and `npm run bundle:windows` builds the NSIS installer locally.

The GitHub release workflow:

1. installs Node.js and Rust on `windows-latest`;
2. installs frontend dependencies;
3. generates platform icons;
4. builds an NSIS installer with `tauri-apps/tauri-action`;
5. creates a **draft** GitHub release; and
6. generates a GitHub artifact attestation for the produced installer.

The release stays draft because repository CI does not contain a Windows code-signing identity. Artifact attestation proves build provenance; it is not a substitute for Windows Authenticode signing or a security audit.

## Acceptance cases

Phase 4 is accepted when automated tests and CI demonstrate at least the following:

- database initialization migrates to schema version 4;
- the database integrity check reports `OK` for a valid database;
- keep-latest-N never selects a primary version inside the retained window;
- exact duplicates are candidates only when the duplicate policy is enabled;
- an unchanged file can archive and restore with the same SHA-256;
- a locally modified file cannot be archived;
- restore refuses to overwrite different bytes at the original path;
- interrupted lifecycle states have deterministic startup-recovery rules;
- the React production build succeeds;
- extension and Tauri configuration validation succeeds;
- `cargo fmt --check`, strict Clippy and Rust tests pass.

## Explicit non-goals

Phase 4 does not add:

- automatic or silent deletion;
- cloud archive/sync;
- bulk cleanup without per-file confirmation;
- browser cookie or authenticated-session capture;
- automatic remote re-download as a substitute for a retained archive;
- overwriting locally modified files;
- malware scanning;
- Windows signing credentials in the repository;
- an AI cleanup score or AI-driven destructive action.
