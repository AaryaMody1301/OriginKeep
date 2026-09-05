# OriginKeep Threat Model

This document covers the local-first OriginKeep desktop application, browser companions, Native Messaging boundary, SQLite Passport store, optional page context, portable passports, remote freshness checks, Trust Lens and recoverable archive.

## Assets to protect

- Local files and locally modified copies.
- Download provenance: source URLs, referrers, timestamps and filenames.
- Optional page title/link/nearby-text context and user notes.
- SHA-256 fingerprints, content locations and version lineage.
- Remote-check evidence and HTTP validators.
- Trust Lens observations and configured Sigstore identity policy.
- The SQLite database and lifecycle ledger.
- Recoverable archive copies.
- Portable passport sidecars when the user creates them.

## Trust boundaries

### Browser companion → Native Messaging host

Browser metadata and optional page context are untrusted input. Native JSON is bounded to 1 MiB and parsed as data. It never grants the companion a general shell-command interface.

Chromium release packages use a deterministic extension origin. Firefox uses an explicit Gecko extension ID. Native-host manifests allow only those configured IDs/origins; wildcard extension origins are not used.

Windows NSIS registers Chrome/Edge and Firefox under the current user. macOS/Linux Browser integration copies the bundled host into a stable per-user location and writes browser-specific manifests with absolute executable paths.

### Optional enhanced page context

Enhanced context requires explicit HTTP/HTTPS host permission from the companion popup. The content script bounds page title, URL, clicked text and nearby text before sending it to extension storage/native messaging.

Threats include accidental capture of sensitive nearby text and overly broad browsing collection. Mitigations:

- host permissions are optional and user-triggered;
- disabling the feature unregisters the content script and removes optional host permissions;
- context is associated with download events rather than used as a general browsing-history feed;
- Safari fallback pending context is bounded to 20 records and a 10-minute adoption window;
- no OriginKeep hosted endpoint receives the context.

### Native host / desktop → filesystem

Recorded paths may become stale, point to replaced files, or collide with files created after the original download. OriginKeep re-hashes bytes before archival and refuses to overwrite different bytes during restore.

Content-location reconnect is hash-gated. A path is added to a content identity only when SHA-256 exactly matches the immutable recorded hash. Similar filenames are not evidence.

### Portable passport → database

A `.originkeep.json` file is user-controlled JSON and is therefore untrusted.

Import mitigations:

- sidecars are limited to 1 MiB;
- the specification identifier must be recognized;
- the adjacent file must exist;
- the adjacent file is hashed locally;
- import is rejected unless the file hash exactly equals the passport SHA-256.

The portable JSON itself is not a digital signature. URLs, notes and cached trust observations may have been edited. Trust evidence should be refreshed after import.

### Existing-file adoption → provenance

Adoption hashes the actual file first. Windows `Zone.Identifier`, macOS `kMDItemWhereFroms`, Safari fallback context and user-provided URLs are evidence sources of different strengths; OriginKeep does not elevate a filename or path into source provenance.

When no source evidence exists, canonical source identity remains unavailable rather than being guessed.

### Desktop → remote source

Remote servers and redirects are untrusted. Freshness checks are explicit user actions and use bounded requests. Authentication failures remain `AUTH_REQUIRED`; network errors remain `CHECK_FAILED`; weak or absent validators never become a guessed `CURRENT` result.

Automatic HTTP redirect following is disabled. Each redirect destination is parsed and validated before a new connection is made. DNS is resolved before connecting, every resolved address must be public, and approved addresses are pinned into the per-hop HTTP client. Loopback, RFC1918/private, carrier-grade NAT, link-local, documentation, benchmark, multicast/reserved and IPv6 local/private destinations are rejected.

Browser cookies, stored credentials and authenticated session headers are not replayed.

### Trust Lens → external local verifier

Trust Lens separates observations rather than producing one trust score.

- Local SHA-256 is computed in-process.
- Windows Authenticode uses local PowerShell/Windows signature APIs.
- C2PA is evaluated only when a local `c2patool` executable is available.
- Sigstore is evaluated only when an adjacent bundle exists, `cosign` is available, and the user configured an expected certificate identity and OIDC issuer.

External verifier output is bounded before storage/display. Missing tools become `VERIFIER_UNAVAILABLE`; missing Sigstore policy becomes `POLICY_REQUIRED`. OriginKeep does not interpret “unsigned” as “malicious.”

### Safari containing-app bridge

Safari Native Messaging reaches a containing Safari app extension rather than Chrome/Firefox-style native-host discovery. The included Swift bridge forwards bounded JSON to the stable per-user OriginKeep host.

Risks include an absent/replaced local host and generated Xcode project drift. The bridge checks for an executable at the expected per-user location and still relies on the Rust host's message bounds/validation. Public Safari distribution remains an Apple signing/Xcode validation boundary.

### Application-data archive

The archive is local storage, not an independent backup service. A machine or disk failure can destroy both the original and archive.

## Destructive-operation threats

### Deleting a locally edited file

Archival requires the current SHA-256 to equal the immutable download fingerprint. A mismatch becomes `LOCAL_MODIFIED` and blocks archival.

### Partial copy followed by original deletion

OriginKeep copies first, flushes the destination, re-hashes it and compares full SHA-256 before removing the original.

### Restore overwrites unrelated data

If the original path exists with a different SHA-256, restore fails. No force-overwrite path is provided.

### Crash during archive or restore

Lifecycle state is persisted as `ARCHIVING` or `RESTORING` before filesystem mutation. Startup reconciliation checks which verified copy survives and chooses an evidence-backed state.

### Archive collision

Archive names include record ID and a SHA-256 prefix. Existing different bytes cause a hard failure.

## Provenance threats

### Signed or expiring download URLs

OriginKeep keeps initiating/canonical source identity distinct from final URL/freshness evidence. Expiration never erases historical provenance.

### Filename-based identity/version guessing

Names such as `final`, `(1)` and `new` are not evidence. Exact equality requires SHA-256; version lineage requires canonical provenance plus content evidence.

### False freshness claims

Equal size alone does not prove `CURRENT`. A first metadata-only remote check establishes/refreshes evidence without claiming freshness. `CURRENT` requires stronger evidence such as HTTP 304 or an unchanged stored validator.

### False authenticity claims

C2PA, Authenticode and Sigstore are evidence about signatures/credentials, not declarations that content is factually true or safe. OriginKeep displays the observation type/state independently.

## Release supply-chain threats

GitHub Actions builds platform bundles from repository source with frozen npm/Cargo resolution. Third-party actions are pinned to full commit SHAs.

CI produces:

- Windows NSIS and browser packages;
- Linux AppImage and Debian package;
- macOS app/DMG smoke build with ad-hoc signing.

Release artifact attestations establish GitHub build provenance but do not replace platform code signing or security review.

Public Windows/macOS/Safari distribution still requires project-owned signing/notarization credentials where applicable. The repository must never commit private signing keys, certificate passwords, browser credentials or access tokens.

## Residual risks

- The local archive usually shares the same physical device as the original; it is not disaster recovery.
- Concurrent external file changes or filesystem/AV policy can interrupt lifecycle operations.
- Authenticated/expiring remote sources may remain unverifiable without user-mediated access.
- PDF comparison covers extracted text layers only, not visual/layout equivalence.
- C2PA/Sigstore tools are optional and their output/trust stores have their own upstream security models.
- OriginKeep does not provide malware detection or sandbox untrusted documents.
- Browser-store IDs can differ from development/package IDs and require allowlist updates before store publication.
- Safari automatic download parity is intentionally not claimed where Safari lacks the equivalent API.
- Unsigned or ad-hoc-signed development/release-candidate binaries can trigger OS trust warnings.

## Security rule

When OriginKeep cannot prove that a destructive lifecycle operation is safe from locally available evidence, it must fail closed and preserve whichever verified copy still exists.
