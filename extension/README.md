# OriginKeep browser companion

The Phase 1 companion uses Chromium Manifest V3 and only requests:

- `downloads` — to receive download metadata such as URL, final URL, referrer, filename, MIME type, timestamps, state, and size.
- `nativeMessaging` — to send that metadata to the local OriginKeep native host.

It does not request permission to read arbitrary page contents or browsing history.

## Development install

1. Build `originkeep-native-host` from `src-tauri`.
2. Load this `extension/` directory as an unpacked extension in Chrome or Edge.
3. Copy the extension ID.
4. On Windows, run `scripts/install-native-host.ps1` with that ID and the absolute path to `originkeep-native-host.exe`.
5. Complete a browser download and open/refresh the OriginKeep desktop app.

Native Messaging requires the host manifest to explicitly allow the installed extension origin. Never use a wildcard allowed origin.
