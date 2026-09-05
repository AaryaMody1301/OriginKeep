# OriginKeep browser companions

OriginKeep ships separate Manifest V3 packages for Chromium-family browsers and Firefox because their background/native-host manifest rules differ.

## Basic provenance permissions

Required permissions are:

- `downloads` — receive browser-supplied download URL/final URL/referrer/path/MIME/time/state/size metadata.
- `nativeMessaging` — send bounded metadata to the local OriginKeep native host.
- `storage` — retain the most recent opt-in download-link context locally inside the extension.
- `scripting` — register the optional context script only after host access has been granted.

The extension does not request cookies, history or authentication-session access.

## Optional rich context

Page-reading access is **not** an install-time requirement. HTTP(S) host access and `tabs` are optional permissions.

Click the OriginKeep companion toolbar action to request richer context. If granted, OriginKeep registers `context-capture.js` for HTTP(S) pages and can attach bounded evidence from a download-link click:

- page URL and title;
- clicked link text;
- bounded nearby text;
- timestamp.

The click context expires after two minutes and is associated conservatively with the download/referrer/origin. Revoking host permissions disables future rich-page capture while basic download provenance remains available.

## Chromium package

Use `manifest.json` for Chrome, Edge and Chromium. The public manifest `key` gives release-package/unpacked installs deterministic extension ID:

`mplmkmbnahpggimgfihfgieamonbbobh`

Native Messaging uses `allowed_origins` with that exact ID; wildcards are not used.

## Firefox package

Release automation copies `manifest.firefox.json` to `manifest.json` inside the Firefox ZIP. Its explicit Gecko ID is:

`originkeep@aaryamody1301.github.io`

Firefox uses a background script/event page rather than Chromium's MV3 service-worker background and its Native Messaging host manifest uses `allowed_extensions`.

## Native-host installation

- Windows: NSIS registers Chrome, Edge and Firefox per-user manifests and removes them on uninstall.
- macOS/Linux: the OriginKeep desktop app creates per-user Chrome/Chromium/Edge/Firefox manifests pointing at its bundled native host when the application starts.

A future browser-store listing may assign a different Chromium-family store ID. The published package and native-host allowlist must agree before store distribution.

## Safari

Safari is not packaged as an automatic-download companion because Apple's current Safari Web Extension tooling does not support the `downloads` manifest capability OriginKeep requires for deterministic download events. See [`../docs/SAFARI.md`](../docs/SAFARI.md).

## Development install

For Windows development builds, `scripts/install-native-host.ps1` remains available when a developer wants to register a locally built host or different Chromium extension ID explicitly.
