# Phase 1 - Provenance foundation

## Goal

Prove the local evidence path end to end:

```text
Chromium download -> Manifest V3 companion -> Native Messaging -> Rust -> SQLite -> Tauri UI
```

## Included in this phase

- Tauri 2 desktop shell with React + TypeScript UI.
- Minimal Chromium extension with `downloads` and `nativeMessaging` permissions only.
- Stable capture identity derived from extension ID, browser download ID, and browser-reported start time.
- Native Messaging host with length-bounded JSON framing.
- Local SQLite schema with idempotent capture upserts.
- Local SHA-256 calculation when a completed file is present.
- Explicit `SOURCE_UNKNOWN` state until remote freshness is actually verified in a later phase.
- `LOCAL_MISSING` when the browser reports completion but the local file cannot be found.
- Search across filename, URL, referrer, and SHA-256.
- Rust tests for hashing and idempotent storage.
- CI for frontend build, extension manifest validation, formatting, clippy, and Rust tests.

## Trust boundary

The browser companion does not read arbitrary page contents. Browser-provided referrer data is treated as provenance evidence when available, not as guaranteed source-page truth. Native messages are untrusted input and are deserialized into a bounded application contract before database writes.

Phase 1 deliberately does **not** claim that a tracked file is remotely current. Freshness evidence, canonical source identities, and version-family decisions belong to later phases.

## Manual Windows smoke test

1. Install Node.js 22+, Rust stable, and the current Tauri Windows prerequisites.
2. Run `npm install`.
3. Run `cargo build --manifest-path src-tauri/Cargo.toml --bin originkeep-native-host`.
4. Load `extension/` as an unpacked extension in Chrome or Edge and copy its extension ID.
5. Run:

   ```powershell
   ./scripts/install-native-host.ps1 -ExtensionId <extension-id> -HostPath ./src-tauri/target/debug/originkeep-native-host.exe
   ```

6. Run `npm run tauri dev`.
7. Complete a browser download.
8. Refresh OriginKeep and verify that the file, origin URL, final URL/referrer when supplied, local path, size, MIME type, and SHA-256 appear.
9. Download another file and confirm a second record appears without modifying the first.
