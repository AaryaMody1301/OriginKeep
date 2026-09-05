# OriginKeep Portable File Passport v1

A portable OriginKeep passport is an adjacent UTF-8 JSON sidecar whose filename is:

```text
<file-name>.originkeep.json
```

The format identifier is:

```text
org.originkeep.passport
```

and the current schema version is `1`.

## Design goals

A portable passport should survive moving the file between folders or machines without turning the JSON into a second copy of the local database.

The invariant is:

```text
passport.file.sha256 == SHA-256(selected file bytes)
```

OriginKeep refuses import when this equality is false.

## Included evidence

The v1 document contains these top-level objects:

- `file` — filename, MIME type when known, byte size and immutable SHA-256.
- `origin` — recorded initiating/final/referrer/source URLs plus optional page title/link/context/browser evidence.
- `lineage` — deterministic version number and exact-duplicate fingerprint when available.
- `intent` — optional purpose, note, expiry/review time and lifecycle policy.
- `evidence` — latest stored remote-state result and check time.

Example shape:

```json
{
  "format": "org.originkeep.passport",
  "version": 1,
  "exportedAt": "2026-09-05 13:00:00",
  "file": {
    "fileName": "report.pdf",
    "mimeType": "application/pdf",
    "bytes": 842031,
    "sha256": "..."
  },
  "origin": {
    "originalUrl": "https://example.org/report.pdf",
    "finalUrl": null,
    "referrer": "https://example.org/reports",
    "sourceIdentity": "https://example.org/report.pdf",
    "pageUrl": "https://example.org/reports",
    "pageTitle": "Annual reports",
    "linkText": "Download report",
    "contextText": "Annual report 2026",
    "browserName": "Firefox",
    "completedAt": "2026-09-05T12:58:00Z"
  },
  "lineage": {
    "versionNumber": 3,
    "duplicateOfSha256": null
  },
  "intent": {
    "purpose": "Reference",
    "note": "Quarterly research",
    "expiresAt": null,
    "retentionPolicy": "REVIEW_WHEN_NEWER"
  },
  "evidence": {
    "latestRemoteState": "CURRENT",
    "latestRemoteCheckedAt": "2026-09-05 13:10:00"
  }
}
```

## Deliberately excluded

Portable passports do not include:

- the OriginKeep SQLite database path;
- the current/previous absolute local filesystem paths;
- archived-file paths;
- browser cookies, auth headers or session credentials;
- file contents;
- a claim that a remote source is current when no deterministic evidence exists.

OriginKeep should redact common credential-bearing query parameters when exporting URL evidence. The local database may retain the browser-reported URL as provenance, but portable/shareable metadata has a stricter privacy boundary.

## Import rules

Import is fail-closed:

1. JSON must use the known format identifier and supported version.
2. The selected local file must exist.
3. OriginKeep recomputes SHA-256 from the selected bytes.
4. The recomputed hash must exactly match `file.sha256`.
5. Only then is provenance/context/intent reconnected.

Filename similarity is never sufficient.

## Trust boundary

The passport JSON is metadata, not a cryptographic signature. SHA-256 binds the document's stated identity to the chosen file bytes, but a malicious actor could create a new passport and matching hash for arbitrary bytes.

Publisher/authorship claims therefore belong in separate verifiable evidence such as a valid C2PA manifest, Authenticode/code-signing evidence, or Sigstore verification against an expected identity.

## Compatibility

Readers should ignore unknown object fields and reject unsupported major/schema versions. Future versions should preserve the `format`, `version`, and `file.sha256` integrity semantics.
