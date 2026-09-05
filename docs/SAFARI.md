# Safari support

Safari differs materially from Chromium and Firefox, so OriginKeep does not claim identical automatic download capture where Safari does not expose the same Downloads API surface.

## Supported OriginKeep flow on macOS

1. Install/run the OriginKeep macOS desktop app.
2. Use **Browser integration** once. OriginKeep copies `originkeep-native-host` to a stable per-user path under `~/Library/Application Support/OriginKeep/bin/` and writes Chrome/Chromium/Edge/Firefox manifests.
3. For Safari, package the shared WebExtension source with Apple's Safari Web Extension packager.
4. Replace the generated `SafariWebExtensionHandler.swift` with [`../safari/SafariWebExtensionHandler.swift`](../safari/SafariWebExtensionHandler.swift).
5. Enable enhanced page context in the companion if desired.
6. Download normally in Safari.
7. Use **Adopt existing file** in OriginKeep for the downloaded file.

OriginKeep then combines evidence conservatively:

- SHA-256 of the actual adopted file;
- macOS `kMDItemWhereFroms` when the OS retained it;
- a recent Safari clicked-page context observation when the Safari bridge delivered one;
- an explicit user source URL when supplied.

If none of these provide a source, the file remains source-unknown.

## Generate the Xcode project

Apple renamed the converter to the Safari Web Extension packager. On a Mac with current Xcode installed:

```bash
./scripts/prepare-safari-project.sh
```

The helper runs the browser package preparation and then uses:

```bash
xcrun safari-web-extension-packager browser-packages/chromium \
  --project-location safari/generated \
  --app-name "OriginKeep Companion" \
  --bundle-identifier "com.originkeep.safari" \
  --swift
```

Review all compatibility warnings printed by Apple's packager. The generated project is intentionally not committed because it is generated Xcode output tied to local Xcode versions.

## Native-message bridge

Safari's `runtime.sendNativeMessage` is delivered to the containing Safari app extension rather than directly discovering the Chrome/Firefox Native Messaging manifest.

The included Swift handler forwards a single bounded JSON message to:

```text
~/Library/Application Support/OriginKeep/bin/originkeep-native-host
```

using the same 32-bit little-endian length-prefixed protocol as the Chromium/Firefox host.

The host stores Safari fallback context locally for a short bounded window so it can be attached when the user adopts the resulting file.

## Why adoption is required

Safari packaging can reuse much of the WebExtension UI/context code, but OriginKeep does not rely on an unsupported Downloads API to learn the final local filename/path. The desktop therefore verifies the real downloaded file during adoption instead of guessing which local file corresponds to a clicked link.

## Distribution boundary

The repository includes the source bridge and packaging helper, but public Safari distribution is an Apple distribution task:

- the generated containing app must be tested in current Xcode/Safari;
- the app/extension needs the appropriate Apple signing identity/team;
- direct distribution requires the applicable notarization path, or the app can be submitted through Apple's distribution channels.

An ad-hoc or local development build is not equivalent to a trusted public Safari release.
