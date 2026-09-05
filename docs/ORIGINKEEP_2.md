# OriginKeep 2.0 — Universal File Passport

OriginKeep 2.0 extends the completed v0.1 provenance/version/freshness/lifecycle engine into a universal local file-memory layer.

## Product statement

> Every file remembers where it came from, why you saved it, whether it changed, and how to get it back.

## Acceptance scope

### 1. Universal File Passport

A tracked file exposes origin, context, SHA-256 identity, lineage, local integrity, latest remote evidence, lifecycle state, purpose/note/expiry, known locations and portable-passport status.

### 2. Files outside the browser flow

Users can adopt an existing local file. OriginKeep fingerprints the bytes and keeps web origin unknown unless the user supplies one. Adopted files participate in duplicate detection, content identity, Passport export, Trust Lens and lifecycle controls.

### 3. Optional save context

Basic browser provenance still works without broad host access. Rich page title/link/nearby text capture requires an explicit user action granting optional host/tab permissions. Captured text is bounded before it reaches Native Messaging.

### 4. Portable passports

Export produces `<file>.originkeep.json`. Import requires exact SHA-256 equality. Absolute local paths and credentials are excluded from the portable format.

### 5. Move/rename identity

Users can relink an explicit candidate path only when SHA-256 matches. A bounded directory scan may find the same content after a move/rename; it is capped by entry count/depth and does not follow symlink directories.

### 6. Origin Graph

The desktop exposes deterministic graph relationships:

```text
SITE -> SOURCE -> FILE
FILE -> NEXT_VERSION -> FILE
FILE -> EXACT_DUPLICATE_OF -> FILE
```

No filename similarity or AI clustering creates lineage edges.

### 7. Intent and expiry

Supported lifecycle intents:

- `MANUAL`
- `REVIEW_WHEN_NEWER`
- `ARCHIVE_WHEN_SUPERSEDED`
- `ARCHIVE_WHEN_EXPIRED`
- `NEVER_ARCHIVE`

Intent modifies safe-cleanup recommendations only. Archive remains explicit and SHA-verified; no silent deletion is introduced.

### 8. Trust Lens

Trust Lens reports independent evidence instead of a score:

- current-vs-baseline SHA-256;
- recorded provenance;
- platform-origin metadata when available;
- platform publisher/code-signature status where meaningful;
- optional C2PA verification through `c2patool`;
- optional Sigstore verification through `cosign`, expected identity and expected OIDC issuer.

Missing tools/evidence remain `UNAVAILABLE`, `NOT_FOUND`, or unverified rather than being inferred.

### 9. Browser reach

- Chrome / Edge / Chromium: automatic download provenance through the Chromium package.
- Firefox: automatic provenance through a Firefox-specific Manifest V3 package and `allowed_extensions` Native Messaging manifest.
- Safari: desktop Passport functionality is supported on macOS, but automatic Safari download-event parity is not claimed because the Safari Web Extension toolchain does not support OriginKeep's required `downloads` manifest capability.

### 10. Desktop reach

CI must build:

- Windows NSIS;
- macOS DMG;
- Linux AppImage and DEB.

The bundled Native Messaging host is target-triple-specific on every desktop platform. macOS/Linux create per-user Chrome/Chromium/Edge/Firefox manifests at app startup; Windows registration remains NSIS-managed.

## Privacy invariants

- Core processing remains local.
- No account/backend/file upload is introduced.
- Browser rich-context permissions are optional.
- No cookies/session tokens are replayed for freshness checks.
- Shareable Passport metadata has a stricter URL-secret redaction boundary than local provenance storage.

## Explicit limitations

- C2PA verification depends on an installed `c2patool`; marker scanning alone never becomes a verified state.
- Sigstore verification depends on an installed `cosign`, an adjacent bundle, expected signer identity and expected issuer.
- Platform code signing/notarization remains an external distribution credential boundary.
- Safari automatic download capture is not available with the current browser API surface.
- Cross-device syncing of the SQLite database is not part of OriginKeep 2.0; portable passports are the interoperability primitive.

## Merge gate

OriginKeep 2.0 is merge-ready only when:

1. frontend TypeScript/build and both browser manifests pass validation;
2. Rust formatting, strict Clippy and all tests pass under the committed lockfile;
3. Windows NSIS with native host builds;
4. macOS DMG with native host builds;
5. Linux AppImage + DEB with native host build;
6. documentation accurately states C2PA/Sigstore/Safari/signing limitations.
