# Phase 2 - Version Intelligence

Phase 2 turns Phase 1 provenance records into deterministic content/version relationships without relying on filename similarity or AI guesses.

## Rules

### Source identity

OriginKeep derives a version-family key from the initiating download URL first, falling back to the final URL only when necessary.

Normalization follows the WHATWG URL model through Rust's `url` crate:

- only `http` and `https` sources receive a canonical identity;
- scheme/host/default-port serialization follows the URL Standard;
- URL fragments are removed because they are not part of the remote HTTP resource request;
- embedded username/password credentials are removed from the identity;
- query parameters are preserved deliberately because removing arbitrary parameters can collapse distinct resources.

Filename similarity is not used to create a family.

### Exact duplicates

SHA-256 is authoritative for exact content equality. A later record with the same SHA-256 points at the earliest prior matching record through `duplicate_of_id`, even when filenames or source URLs differ.

### Version numbers

Within one canonical source identity:

- first distinct fingerprint -> version 1;
- same fingerprint again -> same version number and `DUPLICATE`;
- new fingerprint -> next version number;
- older primary versions become `SUPERSEDED`;
- the newest primary version remains `SOURCE_UNKNOWN` until Phase 3 performs remote freshness verification.

### Local state

The download-time SHA-256 remains immutable evidence. Local verification recomputes the current file hash and stores a separate state:

- `PRESENT`
- `LOCAL_MODIFIED`
- `LOCAL_MISSING`

A local edit never overwrites the original fingerprint.

## Migration

SQLite uses `PRAGMA user_version = 2` after adding/backfilling:

- `source_identity`
- `version_number`
- `duplicate_of_id`
- `local_state`

Existing Phase 1 rows are backfilled in ID order so duplicate ancestry and version numbers are deterministic.

## Acceptance cases

1. Download the same bytes twice under different filenames -> second record is an exact duplicate.
2. Download the same canonical source twice with identical bytes -> both share the same version number; later capture is a duplicate.
3. Download the same canonical source with changed bytes -> new version number; older primary becomes `SUPERSEDED`.
4. Download two URLs that differ only by query value -> they remain separate source identities.
5. Modify a tracked file locally -> `LOCAL_MODIFIED` while the stored download-time SHA-256 remains unchanged.
6. Delete a tracked file locally -> `LOCAL_MISSING`.

## Explicit non-goals

Phase 2 does not claim that a remote source is current or changed. HTTP freshness, ETag/Last-Modified evidence, authentication boundaries, and remote comparisons belong to Phase 3.
