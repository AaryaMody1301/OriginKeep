# OriginKeep 2.0 Release Checklist

This checklist is the release gate for OriginKeep File Passport release candidates. GitHub can prove that an artifact was built from reviewed source; it cannot perform the interactive browser/install acceptance steps or supply project-owned platform signing credentials.

## Build invariants

- `package-lock.json` and `src-tauri/Cargo.lock` are committed and match the 0.2 manifests.
- CI uses `npm ci` and Cargo `--locked` modes.
- GitHub Actions are pinned to full commit SHAs.
- Frontend build and Chromium/Firefox manifest validation pass.
- Browser-specific companion staging produces one valid `manifest.json` per package.
- Rust format, strict Clippy and the full Rust test suite pass.
- Linux desktop code compiles with the Linux Tauri override.
- macOS desktop code compiles with the macOS Tauri override.
- Windows CI successfully builds the real NSIS installer.
- The native host is bundled through Tauri `externalBin`.
- The Windows smoke artifact contains the NSIS installer plus separate Chromium and Firefox companion ZIPs.
- Tag-release automation is configured for Windows NSIS, macOS DMG, Linux DEB/AppImage, Chromium ZIP and Firefox ZIP.
- Release artifacts are built from the same Git tag and are covered by GitHub artifact attestations where configured.

## File Passport acceptance

1. Capture a Chrome/Edge/Firefox download from a normal HTTP(S) link.
2. Verify source URL, SHA-256, page title/page URL and link context appear in the File Passport when a context match exists.
3. Verify unrelated browsing/click context does not appear as arbitrary tracked history.
4. Set a purpose, note, review date and retention intent; restart and confirm persistence.
5. Export `file.ext.originkeep.json` and confirm it does not contain the original absolute local path.
6. Copy the asset + passport together and import them; confirm import succeeds only when SHA-256 matches.
7. Modify the asset and confirm passport import fails.
8. Rename/move a tracked file, verify it becomes missing, and relink it at the new path only when SHA-256 matches.
9. Run an explicit move scan against a bounded test directory and verify only exact identities are reconnected.
10. Adopt an existing local file and confirm it enters the normal duplicate/version engine.
11. Import OS provenance and confirm it is displayed as supplemental evidence rather than identity proof.
12. Inspect the Origin Graph and verify source/version/duplicate edges correspond to real database evidence.

## Trust Lens acceptance

1. An unchanged file reports matching SHA-256 integrity.
2. A locally modified file reports the mismatch and does not keep an integrity-success state.
3. Remote evidence mirrors the existing deterministic freshness state rather than deriving a new score.
4. A file without C2PA shows an explicit absent/unavailable result rather than `TRUSTED`.
5. A valid C2PA file distinguishes `VALID_UNTRUSTED` from `TRUSTED` according to the configured trust anchors.
6. A file without an adjacent `.sigstore.json` reports Sigstore as not present.
7. A valid adjacent Sigstore bundle verifies against the already-recorded SHA-256.
8. An invalid bundle/digest mismatch does not produce `VERIFIED`.
9. No Trust Lens result is marketed as malware detection or a global safety verdict.

## Browser acceptance

### Chromium-family

1. Load the Chromium release ZIP.
2. Confirm the development/release-package extension ID is `mplmkmbnahpggimgfihfgieamonbbobh` unless testing a real store-assigned ID.
3. Confirm download provenance/native messaging works.
4. Confirm matched click context reaches only the local OriginKeep app.

### Firefox

1. Load/sign the Firefox release package as appropriate for the test channel.
2. Confirm explicit ID `originkeep@aaryamody.local` for the repository package.
3. Confirm the host manifest uses `allowed_extensions` and native messaging succeeds.
4. Confirm downloads and matched context are captured.
5. Confirm the manifest's `websiteActivity` / `websiteContent` data categories match the actual local-native transfer behavior before AMO submission.

### Safari/macOS

Do not claim automatic Safari download capture under the current API surface.

1. Download/save a file using Safari normally.
2. Use **Create passport** in the OriginKeep desktop app.
3. Confirm the local file is SHA-256 fingerprinted.
4. Where macOS retained it, confirm `kMDItemWhereFroms` provenance is imported.
5. Confirm the resulting Passport supports the same local integrity/version/lifecycle/Trust Lens functionality that applies to an adopted file.

## Clean Windows test

Use a Windows machine or VM that has never run the tested OriginKeep version.

1. Install the NSIS package for the current user.
2. Confirm `originkeep.exe` and `originkeep-native-host.exe` are installed together.
3. Confirm HKCU native-messaging registration exists for Edge, Chrome and Mozilla Firefox.
4. Confirm Chromium host JSON uses `allowed_origins` and Firefox JSON uses `allowed_extensions`.
5. Exercise one real download through each installed supported browser.
6. Verify provenance, SHA, version/duplicate logic, remote freshness and Downloads Review.
7. Verify archive/restore round-trip and collision refusal.
8. Uninstall and confirm browser native-host registrations/manifests are removed.
9. Confirm application data is not silently destroyed by uninstall.

## Clean macOS test

1. Build/install the DMG/app in the intended test environment.
2. Confirm the bundled `originkeep-native-host` is present.
3. Use **Register browser integrations** and verify per-user Chrome/Edge/Firefox native-host manifests point at the installed host.
4. Exercise Chrome/Firefox automatic capture where installed.
5. Exercise Safari/local-adoption + `kMDItemWhereFroms` flow.
6. Verify portable passport export/import and move recovery.
7. Confirm Gatekeeper/signing behavior is documented accurately for the candidate.

## Clean Linux test

1. Test both the intended DEB/AppImage distribution formats where practical.
2. Confirm the bundled native host launches.
3. Use **Register browser integrations** and verify per-user Chromium/Firefox manifests.
4. Exercise automatic capture through installed supported browsers.
5. Exercise local adoption and move recovery.
6. Confirm archive/restore and remote-source security checks.

## Browser-store ID boundary

Repository packages use deterministic development/release IDs. If a browser store assigns a different ID, update the corresponding native-host allowlist and repeat the clean browser/platform acceptance test with the actual store package before claiming store compatibility.

## Signing/notarization boundary

Release candidates can be unsigned/unnotarized and may trigger Windows/macOS/Linux distribution warnings or blocks depending on platform policy. Stable public packages should use project-owned signing/notarization when available.

GitHub artifact attestation proves build provenance; it does not replace Authenticode, Apple signing/notarization, package-repository trust, browser-store review or a security audit.

## Release sequence

1. Merge OriginKeep 2.0 only after all repository CI jobs are green.
2. Tag a reviewed `main` commit with a version beginning `v0.2.0` (for example `v0.2.0-rc.1`).
3. Let the tag workflow create a draft release and attach all platform/browser artifacts.
4. Perform the relevant clean-platform/browser tests above.
5. Fix release-only findings through normal PRs and issue another RC as needed.
6. Publish a stable `v0.2.0` only after the intended distribution targets pass acceptance and the README/store descriptions match reality.
