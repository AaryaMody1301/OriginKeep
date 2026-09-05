# Safari support boundary

OriginKeep's desktop Passport layer runs on macOS, but the browser companion has an important Safari limitation.

## What Safari Web Extensions provide

Safari Web Extensions use familiar WebExtension technologies and can share JavaScript/content-script logic with Chromium/Firefox projects. Safari also supports messaging between a Safari Web Extension and its containing macOS app/native app extension.

## Why automatic OriginKeep download capture is different

OriginKeep's automatic browser provenance pipeline depends on the WebExtension `downloads` capability so the companion can receive deterministic download creation/completion events with browser-reported URL/path metadata.

Apple's current Safari web-extension packaging documentation warns that the `downloads` manifest capability is unsupported. OriginKeep therefore does not ship a Safari package that pretends to provide the same automatic provenance guarantee.

## What still works for Safari users

On macOS, Safari users can use all desktop-native OriginKeep features:

- adopt any existing local file;
- create/edit File Passports;
- export/import portable `.originkeep.json` passports;
- relink moved/renamed files by SHA-256;
- use Origin Graph, versioning and duplicate detection;
- perform explicit safe remote freshness checks for public sources;
- inspect platform/C2PA/Sigstore trust evidence where tools/evidence are available;
- archive/restore through the safe lifecycle.

When macOS already has `kMDItemWhereFroms` metadata for a file, the Trust Lens can report that platform-origin evidence independently of Safari extension capture.

## Future Safari path

If Apple exposes a supported download-event API suitable for this workflow, the shared context-capture code can be wrapped in a Safari containing app and connected through its native app-extension messaging bridge.

Current Safari Web Extension projects can be generated with Apple's `safari-web-extension-packager`, but producing/signing/notarizing a distributable Safari-containing app still depends on Apple's Xcode/developer tooling and credentials.

Until the browser capability changes, this repository treats Safari as:

```text
macOS desktop Passport support: YES
automatic Safari download-event provenance parity: NO
```
