# OriginKeep 2.0 Release Checklist

This checklist is the release gate for the first public **OriginKeep 2.0 Universal File Passport** release candidate and stable release.

## Automated build invariants

The implementation is release-candidate ready only when all of the following pass from the same reviewed commit:

- `package-lock.json` and `src-tauri/Cargo.lock` are committed.
- CI uses `npm ci` and Cargo `--locked` modes.
- GitHub Actions are pinned to full commit SHAs.
- frontend TypeScript/build passes;
- Chromium and Firefox companion manifests/package validation passes;
- Rust formatting, strict Clippy and the full Rust test suite pass;
- Windows NSIS builds with the bundled target-specific Native Messaging host;
- macOS DMG builds with the bundled target-specific host;
- Linux AppImage and DEB build with the bundled target-specific host.

The OriginKeep 2.0 implementation PR passed all of these gates before merge. A release tag must still rebuild the artifacts from the reviewed `main` commit.

## Version and tag gate

The public desktop version is defined by `src-tauri/tauri.conf.json`. The tag-triggered release workflow rejects a tag whose prefix does not match that version and rejects tags that do not point to a commit already contained in `main`.

For OriginKeep 2.0, use release-candidate tags such as:

```text
v2.0.0-rc.1
v2.0.0-rc.2
```

Use `v2.0.0` only after the relevant clean-machine checks pass.

## Windows clean-machine acceptance

Use a Windows 10/11 x64 machine or VM that has never run OriginKeep.

1. Download the NSIS installer and Chromium/Firefox companion ZIPs from the same draft release.
2. Verify the assets correspond to the intended Git tag/commit and inspect GitHub artifact attestations.
3. Install OriginKeep for the current user.
4. Confirm `originkeep.exe` and `originkeep-native-host.exe` are installed together.
5. Confirm Chrome/Edge/Firefox Native Messaging registrations were created for the current user.
6. Load the Chromium companion unpacked and confirm repository package ID `mplmkmbnahpggimgfihfgieamonbbobh`.
7. Load the Firefox companion and confirm add-on ID `originkeep@aaryamody1301.github.io`.
8. Download a small public file and verify origin, local path, size, MIME type, timestamps and SHA-256 appear.
9. Enable rich context explicitly and verify page title/link/nearby context are captured only after permission is granted.
10. Download the same bytes again and verify exact-duplicate evidence.
11. Download changed bytes from the same canonical source and verify deterministic version lineage.
12. Run local verification and confirm unchanged bytes remain `PRESENT`.
13. Modify a tracked file and confirm it becomes `LOCAL_MODIFIED` without replacing the immutable baseline fingerprint.
14. Run a public remote freshness check and verify evidence is stored.
15. Confirm freshness checks reject loopback/private destinations and reject a public redirect into a private destination.
16. Compare supported local text/CSV/PDF-text versions.
17. Export a File Passport and confirm it contains no absolute local path.
18. Import that Passport beside matching bytes and verify exact SHA-256 acceptance.
19. Attempt import beside changed bytes and verify rejection.
20. Adopt an existing local file and confirm unknown origin remains explicit unless supplied by the user.
21. Move/rename a tracked file and verify explicit relink succeeds only after exact SHA-256 equality.
22. Inspect Trust Lens and confirm missing C2PA/Sigstore tooling/evidence is reported as unavailable/unverified rather than trusted.
23. Use Downloads Review to preview cleanup candidates.
24. Archive one unchanged candidate and confirm the original is removed only after the archive copy passes SHA-256 verification.
25. Restore the archived file and verify the bytes return without overwriting conflicting data.
26. Uninstall OriginKeep and confirm Native Messaging registrations/manifests are removed while application data is not silently destroyed.

## macOS clean-machine acceptance

Use a clean macOS machine/VM appropriate for the generated DMG.

1. Verify the DMG and GitHub attestation belong to the intended tag.
2. Install/open the unsigned release candidate and document any expected Gatekeeper warning.
3. Confirm the bundled Native Messaging host is present.
4. Confirm per-user Chrome/Chromium/Edge/Firefox bridge registration is created by OriginKeep.
5. Exercise file adoption, Passport export/import, exact-hash relink, local integrity and archive/restore.
6. Verify `kMDItemWhereFroms` evidence is imported when macOS exposes it.
7. Verify code-signing evidence is reported factually and does not become a malware/safety verdict.
8. Confirm Safari is not presented as having automatic download-event parity; macOS desktop/adoption features remain available.

A trusted public macOS distribution requires project-controlled Apple signing/notarization outside this repository's credential-free build.

## Linux clean-machine acceptance

Test at least one supported Debian/Ubuntu-family environment for the `.deb` and one AppImage-capable environment.

1. Verify `.deb`/AppImage attestations and tag provenance.
2. Launch the application and confirm the bundled native host is available.
3. Confirm per-user Chromium-family/Firefox bridge registration works where those browsers are installed.
4. Exercise browser provenance where supported plus local adoption, Passport export/import, exact-hash relink, freshness evidence and archive/restore.
5. Confirm unknown platform-origin metadata remains unknown rather than inferred.

## Browser-store boundary

Repository companion packages are suitable for unpacked/release-package testing. Store publication is a separate review/distribution step. If a store assigns a different extension identity, update the corresponding Native Messaging allowlist and repeat clean-install acceptance with the actual store package before claiming store compatibility.

## Trust-tool boundary

Core OriginKeep functionality does not require C2PA or Sigstore tooling. Optional verification requires locally installed tools and explicit evidence/policy:

- C2PA: `c2patool`
- Sigstore: `cosign`, adjacent bundle, expected certificate identity and expected OIDC issuer

Missing tools or policy must never be surfaced as verified authenticity.

## Signing boundary

Release candidates may be unsigned and can trigger platform reputation/security warnings. GitHub artifact attestations establish build provenance; they do not replace Windows Authenticode, Apple signing/notarization, browser-store review or a security review.

## Release sequence

1. Merge the OriginKeep 2.0 finalization PR only after CI is green.
2. Tag `v2.0.0-rc.1` from that reviewed `main` commit.
3. Let the tag-triggered workflow create a draft release and build Windows, macOS, Linux, Chromium and Firefox artifacts from the same tag.
4. Perform the clean-machine acceptance checks above on the platforms available to the project.
5. Fix release-only issues through normal PRs and create another RC tag when needed.
6. Tag `v2.0.0` only when the release-candidate acceptance appropriate to the claimed platforms has passed.
7. Publish the stable draft after final asset/signing/distribution review.

## Post-2.0 non-goals

The following remain outside the 2.0 release contract and are not blockers:

- cloud sync or hosted backup;
- credential/cookie replay for authenticated remote sources;
- download acceleration;
- malware scanning;
- silent destructive cleanup;
- AI-driven cleanup decisions;
- automatic Safari download-event parity without the required browser capability.
