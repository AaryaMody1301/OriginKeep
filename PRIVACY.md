# OriginKeep Privacy

OriginKeep is designed as a local-first file provenance, Passport and lifecycle tool.

## Data processed locally

OriginKeep may store the following on the user's device:

- tracked/adopted file names and local paths;
- browser-reported source URL, final URL and referrer when available;
- download timestamps, MIME type and size;
- SHA-256 fingerprints computed locally;
- deterministic source/version/duplicate metadata and known path history;
- optional user purpose, note, expiry/review time and lifecycle intent;
- optional browser page URL/title, clicked-link text and bounded nearby context when richer context access is enabled;
- remote freshness evidence such as HTTP status, ETag, Last-Modified and Content-Length;
- archive/restore lifecycle metadata;
- local comparison summaries for supported text, CSV and PDF text-layer files;
- local Trust Lens results such as platform-origin/signature status and optional verifier output.

The browser companion uses `downloads` and `nativeMessaging` for the core provenance path. Extension `storage` holds short-lived/bounded click context locally. The `scripting` API is present so an optional context script can be registered after the user grants host access.

## Optional rich browser context

HTTP(S) page access and `tabs` are optional permissions, not required for basic download provenance. The companion requests them only after the user clicks its toolbar action to enable richer context.

When enabled, the companion may capture the current/source page URL and title, clicked download-link text and bounded nearby text. The most recent click context is time-bounded and stored locally by the extension before being attached to a matching download capture.

Users can revoke browser host permissions to stop future page-context capture; basic downloads/native-messaging provenance can continue.

OriginKeep does not request cookie or history permissions as part of this feature.

## Data not intentionally uploaded by core functionality

The core application does not require an OriginKeep account or hosted backend. It does not intentionally collect or upload:

- file contents;
- browser cookies;
- saved passwords;
- authenticated web sessions;
- telemetry or analytics;
- advertising identifiers.

## Remote freshness checks

Remote checks happen only after an explicit user action. OriginKeep contacts the recorded public HTTP(S) source to obtain bounded freshness evidence. Local file contents are not uploaded during these checks.

Release builds reject loopback, private, link-local, documentation/reserved and other non-public IP destinations and validate each redirect before connecting. OriginKeep does not replay browser cookies or authentication headers; protected sources remain `AUTH_REQUIRED` when anonymous checking cannot establish freshness.

## Portable File Passports

A portable passport is an adjacent `.originkeep.json` file that the user may choose to copy/share independently of OriginKeep.

It can contain filename/hash/origin/context/lineage/intent/latest-remote-state metadata. It deliberately excludes absolute local filesystem paths, archive locations, cookies and session credentials. Export should redact common credential-bearing URL query parameters before writing shareable metadata.

Import recomputes the selected file's SHA-256 and refuses to reconnect the Passport if the bytes do not match.

Once a user copies or shares an exported Passport outside OriginKeep, that copy is governed by wherever the user sends/stores it.

## Existing-file adoption and moved-file search

Existing-file adoption reads the selected local file to compute SHA-256. A user-provided source URL is optional; OriginKeep does not invent a public source when none is known.

Moved-file discovery is user-triggered, bounded by entry count/depth, skips symlink directories and hashes candidate files locally. It does not upload candidate contents.

## Trust Lens

Platform-origin/signature checks and SHA-256 inspection run locally.

C2PA verification is attempted only through a locally installed `c2patool`; without the verifier, marker scanning is labeled unverified and never upgraded to a verified claim.

Sigstore verification is user-triggered and requires a locally installed `cosign`, an adjacent bundle and user-supplied expected identity/issuer. Cosign may perform the network/trust-root behavior of the installed Cosign version/configuration during verification. OriginKeep passes the artifact path and verification parameters to the local verifier rather than uploading the file to an OriginKeep service.

## Local comparison and archive

Supported version comparison runs on local files. Recoverable archive copies remain in the local OriginKeep application-data directory. OriginKeep is not a cloud backup service.

## Browser companion permissions

Required:

- `downloads` — observe download metadata and completion state;
- `nativeMessaging` — deliver metadata to the local desktop host;
- `storage` — keep bounded extension context locally;
- `scripting` — register the optional context capture script after host permission is granted.

Optional:

- `tabs`;
- HTTP(S) host access.

The companion does not request cookies or browsing-history permissions.

## Retention and deletion

OriginKeep metadata and recoverable archive copies remain on the local machine until the user removes them or uninstalls/cleans the application data. Desktop uninstallers remove browser native-messaging registrations where installer-owned. Local application data may remain so uninstall does not silently destroy a retained archive or provenance database.

## Third parties

OriginKeep has no hosted service that receives user data. A remote website contacted during a user-requested freshness check can observe a normal HTTP request from the user's device and is governed by that site's own privacy practices. Optional local verifier tools are separate software with their own behavior and policies.

## Contact

Questions about this policy can be raised through the public GitHub repository issue tracker.
