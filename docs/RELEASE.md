# OriginKeep 2.0 Release Checklist

This checklist gates an OriginKeep 2.0 release candidate. CI proves that platform packages can be produced; interactive clean-machine checks still validate real browser/OS integration before a stable public release.

## Build invariants

- `package-lock.json` and `src-tauri/Cargo.lock` are committed and release builds use frozen resolution.
- GitHub Actions are pinned to full commit SHAs.
- TypeScript/Vite build succeeds.
- Chromium and Firefox manifests/scripts validate and package separately.
- Rust format, strict Clippy and all Rust tests pass.
- Windows CI builds NSIS with the bundled native host.
- Linux CI builds AppImage and `.deb` packages.
- macOS CI builds `.app`/DMG with ad-hoc CI signing.
- Browser packages and desktop packages are built from the same tag for releases.
- Public release artifact provenance is attested where the workflow supports it.

## File Passport acceptance

Using representative files:

1. Capture a Chrome/Edge/Firefox download and confirm origin URL, final URL/referrer where supplied, local path, MIME/size and SHA-256.
2. Enable enhanced context explicitly and confirm page title/link text/context appears in the Passport.
3. Disable enhanced context and confirm optional HTTP/HTTPS host permission is removed.
4. Set a purpose, note and review/expiry value and restart OriginKeep; confirm they persist.
5. Move a tracked unchanged file, use **Verify & reconnect**, and confirm the new location is accepted only after exact SHA-256 match.
6. Try reconnecting different bytes with the same filename and confirm rejection.
7. Copy identical bytes to a second location and confirm both locations are represented under the same content identity.
8. Export `<file>.originkeep.json` and inspect it.
9. Copy the file + sidecar to a fresh test database/machine and import it; confirm SHA-256 is revalidated.
10. Modify the adjacent file and confirm passport import fails.
11. Confirm Origin Graph contains source → file → content → location edges and deterministic version edges.

## Trust Lens acceptance

1. Refresh Trust Lens on an unchanged file and confirm local SHA-256 is `VERIFIED`.
2. Modify the file and confirm local integrity becomes `MODIFIED` rather than rewriting the baseline.
3. On Windows, inspect a file with and without `Zone.Identifier` and verify explicit present/not-present results.
4. On Windows, inspect signed and unsigned executables and verify Authenticode results are shown independently.
5. Without `c2patool`, confirm C2PA reports `VERIFIER_UNAVAILABLE` rather than a false verification result.
6. With current official `c2patool`, validate a known C2PA asset and inspect the recorded result.
7. With no Sigstore bundle, confirm `NO_BUNDLE`.
8. With a bundle but no identity policy, confirm `POLICY_REQUIRED`.
9. With `cosign`, a known bundle and explicit expected identity/issuer, confirm pass/fail is evidence-based.

## Clean Windows test

1. Install NSIS on a Windows VM that has never run OriginKeep.
2. Confirm `originkeep.exe` and `originkeep-native-host.exe` are installed together.
3. Confirm current-user Native Messaging registration exists for Chrome, Edge and Firefox.
4. Load the Chromium package and confirm deterministic repository package ID `mplmkmbnahpggimgfihfgieamonbbobh`.
5. Load the Firefox package and confirm Gecko ID `originkeep@originkeep.app`.
6. Complete File Passport and lifecycle acceptance above.
7. Uninstall and verify browser Native Messaging registry entries/manifests are removed without silently deleting user archive/database data.

## Clean Linux test

Test at least one mainstream distribution compatible with the release build baseline.

1. Run/install the AppImage or `.deb`.
2. Use **Browser integration** and confirm OriginKeep copies the host into its stable per-user data directory.
3. Confirm user manifests are written for installed Chromium-family browsers and Firefox.
4. Load Chromium/Firefox companion and complete one provenance capture.
5. Move/reconnect/export/import a Passport.
6. Exercise archive/restore on a temporary file.

## Clean macOS test

1. Open the ad-hoc/internal `.app`/DMG build on a test Mac.
2. Use **Browser integration** and verify stable per-user host installation.
3. Test Chrome/Chromium/Edge or Firefox automatic provenance.
4. Adopt a file that has `kMDItemWhereFroms` and confirm retained provenance is imported.
5. Complete Passport/reconnect/export/import acceptance.
6. Before public distribution, repeat with the actual signed/notarized application build.

## Safari test

Safari is an adoption-based compatibility path rather than a claim of Chromium Downloads API parity.

1. Generate the Safari containing project with `./scripts/prepare-safari-project.sh` on current Xcode.
2. Review every compatibility warning from Apple's packager.
3. Replace the generated handler with `safari/SafariWebExtensionHandler.swift`.
4. Build/run the containing app in Xcode and enable the extension in Safari.
5. Enable enhanced context if desired.
6. Click/download a public file, then adopt the downloaded file in OriginKeep.
7. Confirm macOS provenance and recent Safari context are combined only when available.
8. Confirm a file with no usable evidence stays source-unknown.
9. Public Safari distribution requires Apple signing/distribution review after this development check.

## Existing-file adoption

- Windows: verify `Zone.Identifier` HostUrl/ReferrerUrl import when present.
- macOS: verify `kMDItemWhereFroms` import when present.
- Linux/no metadata: verify source remains unknown unless the user explicitly supplies a source URL.
- In every case, verify SHA-256 is computed from the actual adopted file.

## Remote/lifecycle regression

Retain the original v0.1 guarantees:

- exact duplicate/version semantics remain deterministic;
- local modification verification preserves the immutable baseline hash;
- public-only hardened remote freshness checks reject private/loopback/redirect-to-private destinations;
- `AUTH_REQUIRED` never replays browser cookies;
- text/CSV/PDF-text comparison remains local;
- archive re-verifies SHA-256 before removing the original;
- restore refuses conflicting bytes;
- interrupted archive/restore state is reconciled.

## Store/signing boundaries

Browser store IDs must be added to the relevant native-host allowlist if they differ from repository development/package IDs.

Windows Authenticode and Apple code signing/notarization are external credential-backed release steps. GitHub artifact attestation proves build provenance but does not replace platform signing, malware analysis or security review.

## Suggested release sequence

1. Merge OriginKeep 2.0 only after all CI jobs are green.
2. Tag `v0.2.0-rc.1` from reviewed `main`.
3. Let the tag workflow create/update the draft release with Windows/Linux/macOS and browser assets.
4. Run the clean-machine checks above for the platforms you intend to claim publicly.
5. Fix release-only defects through normal PRs and cut another RC if needed.
6. Tag `v0.2.0` only after claimed-platform checks pass.
7. Add platform signing/store submissions independently when project credentials are available.
