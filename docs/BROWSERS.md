# Browser compatibility

OriginKeep 2.0 has two ways to build a File Passport:

1. automatic browser capture where the browser exposes download events plus Native Messaging;
2. local file adoption where browser download events are unavailable or the file predates OriginKeep.

## Chromium-family browsers

The Chromium package uses Manifest V3 with:

- `downloads` for download metadata/completion;
- `nativeMessaging` for the local OriginKeep Rust host;
- `storage` for short-lived session context matching;
- a content script on HTTP(S) pages to remember the clicked link and bounded nearby context.

The companion does not request cookies or history permissions.

Windows NSIS registers the bundled native host for Chrome and Edge. macOS/Linux can register the bundled host using the in-app **Register browser integrations** action.

## Firefox

Firefox uses the same JavaScript capture code but a separate static manifest because current cross-browser Manifest V3 background/native-host details differ.

The Firefox manifest declares:

```text
originkeep@aaryamody.local
```

as its explicit extension ID. The native-host manifest uses `allowed_extensions`, which is Firefox's equivalent of Chromium's `allowed_origins` relationship.

For current AMO manifest requirements, OriginKeep declares `websiteActivity` and `websiteContent` data-collection categories because matched page URL/title/link/context values leave the extension and are sent to the user's local OriginKeep native application. This is a local transfer, but it is still outside the extension and is therefore disclosed rather than mislabeled as `none`.

## Safari

OriginKeep does **not** claim automatic Safari download capture.

Apple's current Safari Web Extension packaging tool warns that the WebExtensions `downloads` manifest key is unsupported. Safari native messaging also targets the web extension's containing app extension rather than an arbitrary external native host using the Chrome/Firefox model.

OriginKeep therefore supports Safari/macOS users through the universal desktop path:

1. download/save the file normally;
2. use **Create passport** in OriginKeep;
3. OriginKeep hashes the local bytes;
4. it imports `kMDItemWhereFroms` evidence via Spotlight metadata when present;
5. the file receives the same Passport, duplicate/version, freshness, Trust Lens and lifecycle capabilities as other adopted files.

This provides a working macOS/Safari path without shipping an extension that pretends unsupported download events are available.

## Context privacy

The content script sends a candidate context message to the extension background only when a user activates an HTTP(S) link. The background keeps recent candidates in session-only storage when the browser supports it, with an in-memory fallback.

Current behavior:

- maximum 30 recent candidate contexts;
- two-minute matching window;
- exact download URL/final URL match preferred;
- referrer/page fallback second;
- matched context is consumed;
- unmatched context is not stored in the OriginKeep database;
- no disk-backed extension storage fallback is used.

A matched context may contain the page URL, page title, clicked link text and up to a bounded amount of nearby text. Users can remove OriginKeep local application data to remove retained Passport metadata.

## Packaging

Run:

```bash
npm run stage:companions
```

This creates browser-specific directories under `artifacts/browser-companions/` so each release ZIP contains exactly one correctly named `manifest.json`.

Release automation publishes separate Chromium and Firefox ZIPs from the same tag as the desktop packages.
