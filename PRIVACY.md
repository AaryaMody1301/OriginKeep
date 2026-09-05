# OriginKeep Privacy

OriginKeep is designed as a local-first download provenance and lifecycle tool.

## Data processed locally

OriginKeep may store the following on the user's device:

- downloaded file names and local paths;
- browser-reported source URL, final URL and referrer when available;
- download timestamps, MIME type and size;
- SHA-256 fingerprints computed locally;
- deterministic source/version/duplicate metadata;
- remote freshness evidence such as HTTP status, ETag, Last-Modified and Content-Length;
- archive/restore lifecycle metadata;
- local comparison summaries for supported text, CSV and PDF text-layer files.

The browser companion uses the `downloads` permission to receive download metadata and `nativeMessaging` to send that metadata to the locally installed OriginKeep native host.

## Data not collected by OriginKeep

The core application does not require an OriginKeep account or hosted backend. It does not intentionally collect or upload:

- file contents;
- browser history outside the download records exposed by the browser downloads API;
- browser cookies;
- saved passwords;
- authenticated web sessions;
- telemetry or analytics;
- advertising identifiers.

## Remote freshness checks

Remote checks happen only after an explicit user action. OriginKeep contacts the recorded public HTTP(S) source to obtain bounded freshness evidence. Local file contents are not uploaded during these checks.

Release builds reject loopback, private, link-local, documentation/reserved and other non-public IP destinations, and validate each followed redirect before connecting.

## Local comparison and archive

Supported version comparison runs on local files. Recoverable archive copies remain in the local OriginKeep application-data directory. OriginKeep is not a cloud backup service.

## Browser companion permissions

- `downloads`: required to observe download metadata and completion state.
- `nativeMessaging`: required to deliver that metadata to the local desktop host.

The companion does not request arbitrary host permissions, page-content access, history access, cookies or tabs permissions.

## Retention and deletion

OriginKeep metadata and recoverable archive copies remain on the local machine until the user removes them or uninstalls/cleans the application data. Uninstalling the Windows application removes the native-messaging registration; local application data may remain so that an uninstall does not silently destroy a user's retained archive or provenance database.

## Third parties

OriginKeep has no hosted service that receives user data. A remote website contacted during a user-requested freshness check can observe a normal HTTP request from the user's device and is governed by that site's own privacy practices.

## Contact

Questions about this policy can be raised through the public GitHub repository issue tracker.
