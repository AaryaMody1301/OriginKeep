# Phase 3 - Living downloads

Phase 3 turns deterministic provenance/version records into explicit, user-triggered freshness evidence and local version comparisons.

## Scope

### Remote freshness

OriginKeep checks only the latest primary version in a deterministic source family. Exact duplicates and superseded primary versions are not remote-check targets.

A remote check is always explicit. The desktop app does not poll sources in the background.

The checker:

1. Uses the canonical HTTP(S) `source_identity` recorded by Phase 2.
2. Sends `HEAD` first with stored `If-None-Match` / `If-Modified-Since` validators when available.
3. Falls back to a one-byte ranged `GET` only when the server rejects `HEAD` with `405` or `501`.
4. Follows at most five redirects and applies bounded connect/request timeouts.
5. Stores every check in append-only `remote_checks` evidence history.
6. Never uploads the local file.

### Remote evidence states

- `CURRENT`: HTTP `304`, or a successful response with an unchanged ETag / Last-Modified validator.
- `CHANGED`: a successful response with a changed ETag / Last-Modified validator, or a changed remote Content-Length when that is the only comparable evidence.
- `AUTH_REQUIRED`: HTTP `401` or `403` from an anonymous check.
- `SOURCE_MISSING`: HTTP `404` or `410`.
- `CHECK_FAILED`: network failure or an HTTP status that does not provide usable freshness evidence.
- `SOURCE_UNKNOWN`: a successful check that establishes/refreshes a remote baseline but has no prior validator strong enough to make a freshness claim.

The first successful check is intentionally conservative. OriginKeep does not claim `CURRENT` merely because a remote resource exists or has the same size as the local file.

## Evidence history

Phase 3 adds an append-only `remote_checks` table containing:

- download record ID
- check timestamp
- request method (`HEAD` or `GET_RANGE`)
- requested source identity
- final redirected URL
- HTTP status
- resulting evidence state
- ETag
- Last-Modified
- Content-Length
- human-readable reason
- request error, when applicable

The latest evidence is shown in the UI, while the database retains earlier checks for audit/recovery work in Phase 4.

`PRAGMA user_version` advances to `3` after the Phase 3 table is created. Existing Phase 2 rows are not rewritten.

## Local comparison

Comparison is local-only and bounded to 25 MiB per file.

Supported inputs:

- UTF-8 text: line-level additions/removals with a bounded preview.
- CSV: header, row/column count, and changed-cell comparison.
- PDF: text-layer extraction followed by line-level comparison.

PDF comparison is not OCR and is not a visual/layout diff. Image-only PDFs or malformed/unsupported text layers return an explicit error instead of inventing a comparison.

Comparison operates on the local bytes that are currently present. The UI exposes it as a **local comparison**, separate from remote freshness evidence.

## Acceptance cases

1. First remote check with an ETag stores a baseline and remains `SOURCE_UNKNOWN`.
2. A later `304` becomes `CURRENT`.
3. A later successful response with a changed ETag becomes `CHANGED`.
4. HTTP `401`/`403` becomes `AUTH_REQUIRED`.
5. HTTP `404`/`410` becomes `SOURCE_MISSING`.
6. A request error becomes `CHECK_FAILED` and retains the last known validators for future conditional checks.
7. `HEAD` rejection with `405`/`501` retries with `Range: bytes=0-0` rather than downloading the full source.
8. Exact duplicates cannot be remote-checked independently.
9. Superseded primary versions cannot be used as the family freshness target.
10. Text comparison reports deterministic added/removed line counts.
11. CSV comparison reports deterministic schema/shape/cell differences.
12. PDF comparison uses only locally extracted text and never uploads the document.
13. Unsupported file types fail explicitly.
14. Files larger than the Phase 3 comparison bound fail explicitly.

## Non-goals

- background polling or scheduled remote checks
- authenticated browser-session replay
- cloud file upload
- automatic replacement of local files
- visual PDF diff or OCR
- semantic/AI-generated change summaries
- treating filename similarity as version evidence
- claiming freshness from file size alone

These remain intentionally outside Phase 3 so the evidence model stays deterministic and explainable.
