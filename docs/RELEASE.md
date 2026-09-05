# OriginKeep v0.1 Release Checklist

This checklist is the release gate for the first public release candidate and stable v0.1 release.

## Build invariants

- `package-lock.json` and `src-tauri/Cargo.lock` are committed.
- CI uses `npm ci` and Cargo `--locked` modes.
- GitHub Actions are pinned to full commit SHAs.
- Frontend build, extension manifest validation, Rust format, strict Clippy and Rust tests pass.
- A Windows CI job successfully builds the NSIS installer.
- The native host is prepared with the current Windows target triple and bundled through Tauri `externalBin`.
- The companion extension is packaged from the same commit/tag as the installer.

## Clean Windows release-candidate test

Use a Windows machine or VM that has never run OriginKeep.

1. Download the draft/release-candidate NSIS installer and companion ZIP from the same GitHub release.
2. Verify the release assets came from the intended GitHub tag/commit and inspect the GitHub artifact attestation for the installer.
3. Install OriginKeep for the current user.
4. Confirm `originkeep.exe` and `originkeep-native-host.exe` are installed together.
5. Confirm the installer created the Edge and Chrome `NativeMessagingHosts\\com.originkeep.host` HKCU registry entries.
6. Extract the companion ZIP and load it unpacked in Edge or Chrome.
7. Confirm the extension ID is `mplmkmbnahpggimgfihfgieamonbbobh` for the repository release package.
8. Download a small public file.
9. Open/refresh OriginKeep and verify provenance, local path, size, MIME type and SHA-256 appear.
10. Download the same bytes again and verify exact-duplicate evidence.
11. Download a changed file from the same stable source and verify version lineage.
12. Run **Verify local files** and confirm unchanged bytes remain `PRESENT`.
13. Trigger a public remote freshness check and verify evidence is stored.
14. Verify freshness checks reject loopback/private destinations and do not follow a public redirect into a private address.
15. Compare supported local versions for text/CSV/PDF-text content.
16. Use **Downloads Review** to preview cleanup candidates.
17. Archive one unchanged candidate and verify the original path is removed only after the local archive copy passes SHA-256 verification.
18. Restore it and verify the bytes return to the original path.
19. Create different bytes at a restore destination and verify OriginKeep refuses to overwrite them.
20. Uninstall OriginKeep and verify the native-messaging registry entries and host manifest are removed.
21. Confirm local application data is not silently destroyed by uninstall.

## Browser-store boundary

The repository release package uses a deterministic manifest key so an unpacked extension has a stable ID. If Microsoft Edge Add-ons or Chrome Web Store assigns a different ID, update the native host `allowed_origins` list and repeat the clean-install test with the real published package before claiming store compatibility.

## Signing boundary

Release candidates may be unsigned and can trigger Windows reputation warnings. A stable installer should be Authenticode-signed when project-owned signing is available. GitHub artifact attestation proves build provenance; it does not replace code signing or a security review.

## Release sequence

1. Merge the release-candidate hardening PR only after CI is green.
2. Tag `v0.1.0-rc.1` from the reviewed `main` commit.
3. Let the tag-triggered workflow create a draft release with the NSIS installer and companion ZIP.
4. Perform the clean Windows checklist above.
5. Fix any release-only issue through a normal PR and create another RC tag if needed.
6. Tag `v0.1.0` only when the checklist passes.
7. Publish the stable draft after final asset/signing review.

## Deferred post-v0.1 work

The following are intentionally not release blockers:

- scheduled/background freshness checks;
- change notifications and update digests;
- authenticated source sessions;
- cloud sync or backup;
- automatic remote re-download;
- bulk destructive cleanup;
- AI-generated cleanup decisions.
