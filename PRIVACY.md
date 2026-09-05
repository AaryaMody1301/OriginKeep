# OriginKeep Privacy

OriginKeep is designed as a local-first file provenance, File Passport and lifecycle tool.

## Data processed locally

OriginKeep may store the following on the user's device:

- downloaded/adopted file names and local paths;
- browser-reported source URL, final URL and referrer when available;
- download timestamps, MIME type and size;
- SHA-256 fingerprints computed locally;
- deterministic source/version/duplicate metadata;
- matched browser download context: page title, page URL, clicked-link text and bounded nearby text;
- the browser/capture mechanism label;
- optional user note, purpose, review/expiry value and retention intent;
- location history for a tracked file after explicit relinking or move recovery;
- imported operating-system provenance such as Windows Zone.Identifier or macOS `kMDItemWhereFroms` evidence when the user requests it or adopts an existing file;
- remote freshness evidence such as HTTP status, ETag, Last-Modified and Content-Length;
- archive/restore lifecycle metadata;
- local comparison summaries for supported text, CSV and PDF text-layer files;
- local Trust Lens results derived from origin, integrity, remote evidence, OS provenance, C2PA and optional Sigstore evidence.

The core application database and recoverable archive remain local. OriginKeep does not require an OriginKeep-hosted account or backend.

## Browser companion context

Chrome/Edge/Firefox companions use the `downloads` permission to receive download metadata and `nativeMessaging` to send matched metadata to the locally installed OriginKeep native host.

To answer "Why did I download this?", the companion content script observes activation of HTTP(S) links and sends a bounded candidate context to the extension background. The background:

- keeps at most 30 recent candidates;
- expires candidates after approximately two minutes;
- uses session-only extension storage when available, with memory-only fallback;
- prefers exact download URL/final URL matching and may use referrer/page matching as a fallback;
- consumes matched context;
- does not write unmatched context to OriginKeep's database;
- does not use persistent extension local storage as a fallback.

A matched context can contain page URL/title, clicked-link text and bounded nearby text. Because those values leave the browser extension and are transferred to the user's local native application, the Firefox manifest truthfully declares the current AMO `websiteActivity` and `websiteContent` data categories.

## Data not intentionally collected by OriginKeep

OriginKeep does not intentionally collect or upload to an OriginKeep service:

- file contents;
- full browsing history unrelated to matched download context;
- browser cookies;
- saved passwords;
- authenticated web sessions;
- authorization headers;
- telemetry or analytics;
- advertising identifiers.

There is no OriginKeep cloud service receiving this data in the current architecture.

## Portable File Passports

A user can explicitly export a sibling `.originkeep.json` file. Portable passports intentionally exclude absolute local filesystem paths, local location history, archive paths and the local database ID.

They can include source/referrer/page URLs, page context, browser label, SHA-256, notes, purpose and retention intent. Those values may themselves contain personal or sensitive information. Users should review an exported passport before sharing it with another person or service.

Import accepts a portable passport only after the adjacent file matches the recorded SHA-256.

## File move scanning

Move recovery runs only after the user supplies a directory. OriginKeep scans local files under that directory up to a configured cap, uses recorded size as a prefilter, and hashes candidate bytes locally. It does not upload scan results or file contents.

## Remote freshness checks

Remote checks happen only after an explicit user action. OriginKeep contacts the recorded public HTTP(S) source to obtain bounded freshness evidence. Local file contents are not uploaded during these checks.

Release builds reject loopback, private, link-local, documentation/reserved and other non-public IP destinations, and validate each followed redirect before connecting. OriginKeep does not replay cookies or authenticated browser sessions; protected sources can remain `AUTH_REQUIRED`.

## Trust Lens

Trust Lens is local evidence inspection, not an antivirus verdict or generalized trust score.

- SHA-256 integrity reads the local file.
- C2PA inspection reads local asset metadata/manifests.
- Sigstore inspection occurs only when an adjacent `<file>.sigstore.json` bundle exists and verifies it against the recorded artifact SHA-256.
- the embedded Sigstore production trusted-root snapshot is shipped with the dependency graph; OriginKeep does not contact Sigstore merely to display Trust Lens evidence.

A cryptographically valid signature/provenance credential does not by itself prove that the file is safe, true or appropriate for the user.

## Local comparison and archive

Supported version comparison runs on local files. Recoverable archive copies remain in the local OriginKeep application-data directory. OriginKeep is not a cloud backup service.

## Browser companion permissions

Chromium and Firefox packages request:

- `downloads`: observe download metadata and completion state;
- `nativeMessaging`: deliver matched provenance/context to the local desktop host;
- `storage`: maintain short-lived session context needed to associate a clicked link with a later download event.

The companions do not request cookie or history permissions.

Safari is different: OriginKeep does not claim automatic Safari download capture because Safari's current WebExtensions tooling does not support the `downloads` manifest key. Safari/macOS users can adopt files in the desktop app and import macOS provenance when available.

## Retention and deletion

OriginKeep metadata and recoverable archive copies remain on the local machine until the user removes them or uninstalls/cleans the application data. Uninstalling the Windows application removes the native-messaging registration; local application data may remain so that uninstall does not silently destroy a user's retained archive or provenance database.

Retention intent in a File Passport is metadata. OriginKeep 2.0 does not silently archive/delete a file solely because an intent or review date is present.

## Third parties

OriginKeep has no hosted service that receives user data. A remote website contacted during a user-requested freshness check can observe a normal HTTP request from the user's device and is governed by that site's own privacy practices.

## Contact

Questions about this policy can be raised through the public GitHub repository issue tracker.
