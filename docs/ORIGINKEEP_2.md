# OriginKeep 2.0 — Universal File Passport

OriginKeep 2.0 expands the project from a Windows download-provenance manager into a local-first memory layer for files.

Its core promise is:

> Every file remembers where it came from, why you saved it, whether it changed, and how to get it back.

## Passport dimensions

Each tracked file can expose eight independent evidence dimensions:

1. **Origin** — original URL, final URL, referrer, browser and download time.
2. **Context** — page title, page URL, clicked link text and nearby page text when the user enables enhanced context.
3. **Identity** — immutable SHA-256 and all verified local locations carrying those exact bytes.
4. **Integrity** — whether current local bytes still match the download-time hash.
5. **Freshness** — conditional HTTP evidence from the source, including explicit unknown/auth-required states.
6. **Authenticity** — available Windows Authenticode, Windows origin metadata, C2PA and Sigstore evidence.
7. **Lineage** — deterministic source family, version number and exact duplicate ancestry.
8. **Recovery** — current path, archive state and collision-safe restore ability.

No single AI or trust score combines these dimensions. Evidence remains inspectable.

## Browser capability matrix

| Browser | Automatic download metadata | Native Messaging | Enhanced clicked-page context | Notes |
| --- | --- | --- | --- | --- |
| Chrome | Yes | Yes | Optional | First-class Chromium package |
| Edge | Yes | Yes | Optional | First-class Chromium package |
| Chromium | Yes | Yes | Optional | User-level native-host registration on Linux/macOS |
| Brave / Vivaldi | Chromium API dependent | Registered on Linux/macOS | Optional | Same Chromium companion package where supported |
| Firefox | Yes | Yes | Optional | Separate MV3 manifest and explicit Gecko ID |
| Safari | No equivalent full Downloads API path | Through containing Safari app | Optional fallback | Use macOS adoption + OS provenance; see `SAFARI.md` |

Enhanced context requires an explicit user permission grant for HTTP/HTTPS pages. Basic Chrome/Edge/Firefox download provenance does not require page host permissions.

## Desktop capability matrix

| Platform | Desktop bundle | Browser-host installation | OS provenance import |
| --- | --- | --- | --- |
| Windows | NSIS | Installer-managed Chrome/Edge/Firefox HKCU registration | Zone.Identifier / Mark-of-the-Web |
| Linux | AppImage + `.deb` | Per-user stable native-host copy and manifests | No generic OS provenance source is assumed |
| macOS | `.app` + DMG | Per-user stable native-host copy and manifests | `kMDItemWhereFroms` when available |

CI uses ad-hoc macOS signing only to prove the bundle can be produced. Public macOS distribution still requires Apple signing/notarization.

## Existing-file adoption

OriginKeep can hash and adopt a file that predates OriginKeep.

- Windows reads `Zone.Identifier` when present.
- macOS reads `kMDItemWhereFroms` when present.
- Safari fallback context can be combined with the most recent local adoption window.
- Linux and files without retained OS metadata are accepted with `SOURCE_UNKNOWN` semantics unless the user provides a known source URL.

Source identity is never inferred from filename similarity.

## Content identity and locations

`file_locations` is keyed by SHA-256 plus path. Moving, renaming or copying a file does not change its content identity.

The reconnect command:

1. hashes the candidate path;
2. compares it to the immutable baseline SHA-256;
3. records the new path only on exact match;
4. repairs the primary path only when the previous primary is missing.

A different file with a similar name is rejected.

## Portable Passports

`<file>.originkeep.json` exports provenance and user context beside the file. Import re-hashes the adjacent file before accepting the sidecar.

See [`PASSPORT_SPEC.md`](PASSPORT_SPEC.md).

## Trust Lens

Trust Lens deliberately reports separate observations:

- local SHA-256 integrity — built in;
- Windows Mark-of-the-Web / Zone.Identifier — Windows only;
- Authenticode — evaluated through Windows PowerShell;
- C2PA Content Credentials — evaluated through the official `c2patool` when locally installed;
- Sigstore — evaluated through `cosign verify-blob` when a bundle and an explicit expected identity/issuer policy are provided.

Missing verifier tools produce `VERIFIER_UNAVAILABLE`. Missing Sigstore identity policy produces `POLICY_REQUIRED`. OriginKeep does not turn absence of evidence into either a security failure or a verified claim.

## Origin Graph

The graph is derived from deterministic stored relationships:

```text
SOURCE --PRODUCED--> FILE --HAS_CONTENT--> SHA-256 --LOCATED_AT--> PATH
                    |
                    +--NEXT_VERSION--> FILE
                    +--SAME_CONTENT--> SHA-256
```

It is intentionally not a semantic/AI graph. Every edge corresponds to stored provenance, hash identity or version evidence.

## Purpose and expiry

Purpose is optional local metadata:

`REFERENCE`, `READ_LATER`, `TEMPORARY`, `WORK`, `RECEIPT`, `INSTALLER`, `DATASET`, `OTHER`, or `UNSPECIFIED`.

Expiry/review text is advisory. It can help future cleanup and review workflows but does not authorize automatic deletion.

## Authentication boundary

OriginKeep does not copy browser cookies, session tokens or authenticated request headers. Sources that cannot be checked anonymously stay `AUTH_REQUIRED`/unknown rather than receiving a false freshness result.

## Non-goals retained

OriginKeep 2.0 is still not:

- an antivirus;
- an AI chatbot;
- a cloud drive;
- a browser-history sync service;
- a download-speed manager;
- a generic automatic folder organizer;
- an automatic destructive cleanup agent.
