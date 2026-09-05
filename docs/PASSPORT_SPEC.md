# OriginKeep Portable Passport v1

OriginKeep Portable Passport is a local JSON sidecar format that keeps a file's provenance and user context portable when the file is copied to another machine.

The current specification identifier is:

```text
org.originkeep.passport.v1
```

A passport is written beside the file using this filename convention:

```text
report.pdf
report.pdf.originkeep.json
```

## Design goals

A passport must be:

- portable without an OriginKeep account or cloud service;
- linked to exact file bytes by SHA-256 rather than filename;
- human-readable JSON;
- conservative about authenticity and freshness;
- safe to import without trusting paths or cached claims blindly.

## Example

```json
{
  "spec": "org.originkeep.passport.v1",
  "exportedAt": "2026-09-05 09:30:00",
  "sha256": "<64 hex characters>",
  "fileName": "report.pdf",
  "mimeType": "application/pdf",
  "bytes": 1483920,
  "originalUrl": "https://example.com/reports/report.pdf",
  "finalUrl": "https://cdn.example.com/report.pdf",
  "referrer": "https://example.com/reports",
  "sourceIdentity": "https://example.com/reports/report.pdf",
  "downloadedAt": "2026-09-05T09:20:10Z",
  "versionNumber": 2,
  "browserName": "Firefox",
  "pageTitle": "Annual reports",
  "pageUrl": "https://example.com/reports",
  "linkText": "Download annual report",
  "contextText": "Annual report 2026 …",
  "contextSource": "enhanced-click",
  "purpose": "REFERENCE",
  "note": "Used for quarterly research",
  "expiresAt": null,
  "trust": []
}
```

Fields may be `null` when evidence was not available. OriginKeep does not invent missing provenance.

## Import invariant

Import is accepted only when all of the following hold:

1. the sidecar is at most 1 MiB;
2. `spec` is a supported specification identifier;
3. the sidecar filename ends in `.originkeep.json`;
4. the adjacent file exists;
5. SHA-256 of the adjacent file exactly equals the passport `sha256` value.

If the hash does not match, OriginKeep rejects the import. A similar filename is never treated as identity.

## Trust semantics

The passport is a portability container, not a digital signature. A user can edit JSON metadata manually. The immutable file hash proves which bytes the passport refers to, but the JSON itself is not proof that a URL, note or cached trust observation is truthful.

Trust observations exported in the passport are historical evidence. OriginKeep's Trust Lens should be refreshed on the receiving machine before relying on current Authenticode, C2PA, Sigstore, remote-source or local-integrity state.

## Purpose values

Current deterministic purpose values are:

```text
UNSPECIFIED
REFERENCE
READ_LATER
TEMPORARY
WORK
RECEIPT
INSTALLER
DATASET
OTHER
```

Purpose and expiry are user metadata. They never authorize automatic destructive cleanup.

## Compatibility

Unknown future fields should be ignored by readers when the `spec` version remains compatible. A new incompatible format will receive a new specification identifier instead of silently changing v1 semantics.

## Privacy

A portable passport can contain URLs, referrers, page titles, nearby clicked-link context and user notes. Export is always an explicit local action. Users should review a sidecar before sharing it with another person because the metadata can reveal browsing context even when the underlying file itself does not.
