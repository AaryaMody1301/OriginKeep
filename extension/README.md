# OriginKeep browser companion

The OriginKeep companion uses Chromium Manifest V3 and only requests:

- `downloads` — to receive browser-supplied download metadata such as URL, final URL, referrer, filename, MIME type, timestamps, state, and size.
- `nativeMessaging` — to send that metadata to the local OriginKeep native host.

It does **not** request permission to read arbitrary page contents, browsing history, cookies, or authentication sessions.

## Release-package identity

The manifest contains a public `key` so unpacked/release-package installs use the deterministic extension ID:

`mplmkmbnahpggimgfihfgieamonbbobh`

The Windows NSIS installer registers the bundled native host for exactly that extension origin in both Edge and Chrome. No wildcard origin is used.

A future Microsoft Edge Add-ons or Chrome Web Store listing may receive a different store ID. If that happens, the native-host `allowed_origins` list must be updated to include the real published ID before that store package is released.

## Release candidate install

1. Install the OriginKeep Windows package.
2. Extract the `OriginKeep-Companion-*.zip` release asset.
3. In Edge or Chrome, enable Developer mode and choose **Load unpacked**.
4. Select the extracted companion directory and verify its ID is `mplmkmbnahpggimgfihfgieamonbbobh`.
5. Complete a browser download and open or refresh OriginKeep.

The desktop installer already includes and registers `originkeep-native-host.exe`; no separate native-host build is needed for a release-package install.

## Development install

For development builds, `scripts/install-native-host.ps1` remains available when a developer wants to register a locally built host or a different extension ID explicitly.

Native Messaging requires the host manifest to explicitly allow every installed extension origin. Never use a wildcard allowed origin.
