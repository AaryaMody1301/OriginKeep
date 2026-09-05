# OriginKeep 2.0 — Universal File Passports

OriginKeep 2.0 expands the v0.1 download-provenance engine into a local-first memory and evidence layer for everyday files.

The product promise is:

> Every file remembers where it came from, why you saved it, whether it changed, and how to get it back.

The File Passport model keeps these concerns separate instead of collapsing them into an opaque score:

| Passport section | Question | Evidence source |
| --- | --- | --- |
| Origin | Where did this come from? | Browser download metadata, source URL, OS provenance |
| Context | Why did I save it? | Page title, page URL, clicked link text, bounded nearby text, user note |
| Identity | Is this the same file? | SHA-256 |
| Integrity | Have my local bytes changed? | Current SHA-256 vs immutable download/adoption SHA-256 |
| Authenticity | Is there cryptographic publisher/provenance evidence? | C2PA and optional adjacent Sigstore bundle |
| Freshness | Is the recorded remote source unchanged? | Conditional HTTP validators and explicit HTTP evidence |
| Lineage | Which version or duplicate is this? | Canonical source identity plus exact SHA evidence |
| Recovery | Can I safely archive/restore it? | Local lifecycle ledger and SHA verification |

## What is new

### Download context

The Chromium/Firefox companions retain recent clicked-link context in browser session storage only long enough to match it to a real browser download. A matched capture may include:

- page title;
- page URL;
- clicked link text;
- bounded nearby text;
- browser name.

Unmatched browsing context is not persisted in OriginKeep.

### Portable passports

`Export passport` writes a sibling file:

```text
report.pdf
report.pdf.originkeep.json
```

The portable file deliberately excludes absolute local filesystem paths. Import succeeds only when the adjacent asset hashes to the passport's recorded SHA-256.

See [`PASSPORT_SPEC.md`](PASSPORT_SPEC.md).

### Move and rename recovery

A file is not identified by its name. A user can:

- relink one missing record to a new path;
- scan an explicitly chosen directory for missing tracked files.

OriginKeep uses stored size as a prefilter and requires the exact recorded SHA-256 before changing identity/location metadata. Previous locations remain in the local history.

### Adopt existing files

`Create passport` accepts an existing local file, hashes it, imports available operating-system provenance, and passes it through the same duplicate/version engine as browser downloads.

This makes OriginKeep useful for files obtained before installation and provides the practical Safari/macOS bridge described in [`BROWSERS.md`](BROWSERS.md).

### Trust Lens

Trust Lens is evidence, not a malware verdict or AI safety score. It displays independent signals:

- browser/source origin evidence;
- local SHA-256 integrity;
- latest remote freshness evidence;
- imported operating-system provenance;
- C2PA validation state when a readable manifest exists;
- Sigstore verification when an adjacent `<file>.sigstore.json` bundle exists.

A C2PA `VALID_UNTRUSTED` result means the cryptographic assertions validate but the signer does not chain to a configured trust anchor. It must not be presented as equivalent to `TRUSTED`.

Sigstore verification is performed against the already-recorded artifact SHA-256 and the embedded production trust-root snapshot shipped by the Sigstore Rust trust-root crate. No claim is made that every software publisher uses Sigstore.

### Origin Graph

The graph is deterministic. Nodes and edges come from stored evidence:

```text
SOURCE --ORIGIN--> FILE
FILE v1 --NEXT_VERSION--> FILE v2
FILE --EXACT_DUPLICATE--> FILE COPY
```

Filename similarity is not a graph edge.

### Purpose, note, expiry and retention intent

A passport can store an optional purpose:

`Reference` · `Read later` · `Temporary` · `Work` · `Receipt` · `Installer` · `Dataset` · `Other`

It can also store a user note, review/expiry date, and retention intent. Retention intent is metadata in 2.0; OriginKeep does not silently perform a destructive/archive action just because an intent is set.

## Browser/platform matrix

| Platform | Chrome | Edge | Firefox | Safari |
| --- | --- | --- | --- | --- |
| Windows | automatic provenance + context | automatic provenance + context | automatic provenance + context | n/a |
| macOS | automatic provenance + context | automatic provenance + context | automatic provenance + context | local adoption + macOS provenance |
| Linux | automatic provenance + context | automatic provenance + context where installed | automatic provenance + context | n/a |

Safari is intentionally not described as automatic-download compatible. Apple's current Safari Web Extension packaging tooling reports the WebExtensions `downloads` manifest key as unsupported. OriginKeep therefore uses local adoption plus `kMDItemWhereFroms` provenance when available on macOS instead of shipping a companion that pretends to capture unsupported download events.

## Desktop distribution

The repository contains Tauri configurations for:

- Windows NSIS;
- macOS `.app` / `.dmg`;
- Linux `.deb` / AppImage.

All desktop packages bundle the Rust native host. Windows registration is handled by NSIS. macOS/Linux users can register browser integrations from the OriginKeep UI; an explicit shell helper is also available for development.

Platform code-signing/notarization credentials are not embedded in the repository. Release packages remain drafts until the platform checklist in [`RELEASE.md`](RELEASE.md) is satisfied.

## Authenticated sources

OriginKeep does not replay browser cookies, authorization headers or login sessions. A source that rejects anonymous freshness requests remains `AUTH_REQUIRED`; OriginKeep does not turn lack of evidence into a false `CURRENT` state.

## Explicit non-goals

OriginKeep 2.0 remains intentionally outside these categories:

- antivirus or malware verdict engine;
- cloud drive or mandatory sync service;
- AI chatbot;
- generic download accelerator/media grabber;
- filename-based auto organizer;
- hidden autonomous deletion;
- credential/session collector.
