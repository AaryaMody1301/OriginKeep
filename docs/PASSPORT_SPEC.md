# OriginKeep Portable Passport v1

OriginKeep can export provenance and user-intent metadata beside a file as UTF-8 JSON.

The current schema identifier is:

```text
https://originkeep.local/passport/v1
```

The identifier is a stable format name for this repository version; it is not currently a hosted JSON-Schema URL.

## Naming

For a local file:

```text
report.pdf
```

OriginKeep writes:

```text
report.pdf.originkeep.json
```

Import expects the asset to remain adjacent to the passport and strips exactly the `.originkeep.json` suffix to locate the asset.

## Security invariant

A portable passport never establishes file identity by name or path.

Import MUST:

1. parse a recognized OriginKeep passport schema;
2. locate the adjacent asset;
3. compute the asset SHA-256 locally;
4. compare it with the passport `sha256` value;
5. reject the import on mismatch.

Only after these checks may OriginKeep ingest the provenance metadata.

## Privacy invariant

The portable representation deliberately omits:

- absolute local filesystem paths;
- location history;
- OriginKeep database IDs;
- archive paths;
- operating-system provenance event blobs;
- remote-check history.

Those remain local to the receiving/sending OriginKeep databases.

Users should still review exported passports before sharing them: source URLs, referrers, page URLs, nearby page context, notes, and query parameters may themselves contain personal or sensitive information.

## Fields

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `schema` | string | yes | Exact schema identifier |
| `fileName` | string | yes | Human-readable source file name; not an identity key |
| `sha256` | string | yes | Lowercase SHA-256 content identity |
| `bytes` | integer/null | no | Recorded byte length |
| `originalUrl` | string | yes | Recorded origin/adoption URL |
| `finalUrl` | string/null | no | Browser final URL if supplied |
| `referrer` | string/null | no | Browser/referrer evidence if supplied |
| `sourceIdentity` | string/null | no | Conservative canonical HTTP(S) family identity |
| `downloadedAt` | string/null | no | Original timestamp when available |
| `versionNumber` | integer/null | no | Informational version number in exporting database |
| `pageTitle` | string/null | no | Matched page title |
| `pageUrl` | string/null | no | Matched context page URL |
| `linkText` | string/null | no | Text of clicked download link |
| `contextText` | string/null | no | Bounded nearby text captured at click time |
| `browserName` | string/null | no | Browser/capture mechanism label |
| `userNote` | string/null | no | User-provided note |
| `purpose` | string/null | no | User-provided purpose category |
| `expiresAt` | string/null | no | User review/expiry value |
| `retentionAction` | string | yes | Retention intent |

`versionNumber` is informational when moving between databases. The receiving OriginKeep instance still applies its deterministic source/hash family rules during ingest rather than blindly trusting a foreign version number.

## Example

```json
{
  "schema": "https://originkeep.local/passport/v1",
  "fileName": "annual-report.pdf",
  "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "bytes": 842012,
  "originalUrl": "https://example.com/investors/annual-report.pdf",
  "finalUrl": null,
  "referrer": "https://example.com/investors",
  "sourceIdentity": "https://example.com/investors/annual-report.pdf",
  "downloadedAt": "2026-09-05T08:30:00Z",
  "versionNumber": 3,
  "pageTitle": "Annual Reports",
  "pageUrl": "https://example.com/investors",
  "linkText": "Download annual report",
  "contextText": "Annual report 2026",
  "browserName": "Firefox",
  "userNote": "Used for quarterly research",
  "purpose": "Reference",
  "expiresAt": null,
  "retentionAction": "REVIEW"
}
```

## Compatibility

Unknown future fields should not be treated as evidence by older importers. A future incompatible format must use a new schema identifier rather than silently changing the meaning of v1 fields.
