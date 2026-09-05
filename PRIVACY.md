# OriginKeep Privacy

OriginKeep is designed as a local-first File Passport, provenance and recoverable-lifecycle tool.

## Data processed locally

OriginKeep may store the following on the user's device:

- file names and local paths;
- browser-reported source URL, final URL and referrer when available;
- download timestamps, MIME type and size;
- SHA-256 fingerprints computed locally;
- deterministic source/version/duplicate metadata;
- verified additional locations for the same SHA-256 content;
- optional user purpose, note and review/expiry metadata;
- remote freshness evidence such as HTTP status, ETag, Last-Modified and Content-Length;
- archive/restore lifecycle metadata;
- local comparison summaries for supported text, CSV and PDF text-layer files;
- Trust Lens observations such as local integrity, Windows origin metadata, Authenticode, C2PA or Sigstore verifier results;
- imported OS provenance such as Windows `Zone.Identifier` or macOS `kMDItemWhereFroms` when available.

## Browser companion

Basic automatic provenance on supported Chromium/Firefox browsers uses:

- `downloads` — download metadata and completion state;
- `nativeMessaging` — delivery to the locally installed OriginKeep host;
- `tabs` — browser-reported tab title/URL matching when available;
- `storage` — a short-lived local context handoff between extension components;
- `scripting` — used only to register the optional enhanced-context content script after permission is granted.

### Enhanced page context is opt-in

HTTP/HTTPS host permissions are listed as **optional**, not automatic. OriginKeep asks for them only after the user chooses **Enable enhanced context** in the companion popup.

When enabled, OriginKeep may locally retain bounded context associated with a download:

- page title;
- page URL;
- clicked link/button text;
- nearby page text around the clicked element.

Disabling enhanced context unregisters the content script and removes the optional host permissions.

OriginKeep does not use enhanced context to build a general browsing-history database.

## Safari fallback context

Safari does not provide the same automatic download metadata path used by Chromium/Firefox. When the optional Safari containing-app bridge is used, a clicked-page context observation may be stored locally for a short bounded window so it can be associated when the user explicitly adopts the downloaded file in the macOS desktop app.

The fallback table keeps at most 20 observations and the adoption path only considers a context from the previous 10 minutes. It is not a persistent Safari browsing log.

## Portable File Passports

Exporting `<file>.originkeep.json` is an explicit local action. A portable passport can include URLs, referrers, page title/link context and user notes.

OriginKeep does not upload passport sidecars. If a user chooses to share a passport file with another person or service, that sharing happens outside OriginKeep. Users should review the JSON before sharing because provenance metadata may reveal browsing or work context.

Passport import re-hashes the adjacent file and rejects the sidecar if its SHA-256 does not match.

## Trust Lens tools

Built-in local SHA-256 verification does not contact a network service.

Optional verifier integrations invoke locally installed tools/processes:

- Windows PowerShell for Authenticode;
- official `c2patool` when installed;
- `cosign verify-blob` when installed and when the user has configured an expected signing identity/OIDC issuer.

OriginKeep does not send file bytes to an OriginKeep server. Some verifier tools may have their own behavior or trust-material update mechanisms; users can inspect those projects and their configuration separately.

## Data not collected by OriginKeep

The core application does not require an OriginKeep account or hosted backend. It does not intentionally collect or upload:

- file contents;
- browser cookies;
- saved passwords;
- authenticated web sessions;
- telemetry or analytics;
- advertising identifiers;
- a general browsing-history feed.

## Remote freshness checks

Remote checks happen only after an explicit user action. OriginKeep contacts the recorded public HTTP(S) source to obtain bounded freshness evidence. Local file contents are not uploaded during these checks.

Release builds reject loopback, private, link-local, documentation/reserved and other non-public IP destinations, and validate each followed redirect before connecting.

OriginKeep does not replay browser cookies or authenticated session headers. Sources that require authentication remain `AUTH_REQUIRED`/unknown.

## Local comparison and archive

Supported version comparison runs on local files. Recoverable archive copies remain in the local OriginKeep application-data directory. OriginKeep is not a cloud backup service.

## Retention and deletion

OriginKeep metadata and recoverable archive copies remain on the local machine until the user removes them or uninstalls/cleans the application data. Uninstalling a desktop package can remove native-messaging registration, but local application data may remain so uninstall does not silently destroy a user's retained archive or provenance database.

## Third parties

OriginKeep has no hosted service that receives user data. A remote website contacted during a user-requested freshness check can observe a normal HTTP request from the user's device and is governed by that site's own privacy practices.

## Contact

Questions about this policy can be raised through the public GitHub repository issue tracker.
