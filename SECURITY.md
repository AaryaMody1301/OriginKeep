# Security Policy

OriginKeep handles local file paths, provenance metadata, network freshness checks and reversible file lifecycle operations. Security reports are welcome.

## Supported version

Until the first stable release, the latest code on `main` and the newest published release candidate are the supported versions.

## Reporting a vulnerability

Please avoid posting exploit details, private file paths, credentials or other sensitive user information in a public issue.

Use GitHub's private vulnerability reporting feature when it is available for this repository. If private reporting is unavailable, open a minimal public issue requesting a private contact channel without including exploit details.

A useful report includes:

- affected OriginKeep version or commit;
- operating system and browser;
- whether the issue involves the desktop app, native host or browser companion;
- minimal reproduction steps that do not expose private data;
- expected and observed behavior;
- security impact.

## Security boundaries

The detailed threat model is maintained in [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md). Important invariants include:

- destructive lifecycle operations fail closed unless SHA-256 evidence proves the operation safe;
- restore does not overwrite different bytes;
- native messaging only accepts explicitly allowed extension origins;
- remote freshness checks are user-triggered and restricted to validated public HTTP(S) destinations;
- local file contents are not uploaded by core functionality;
- release signing keys, browser credentials and tokens must never be committed to the repository.

## Release trust

GitHub artifact attestations establish build provenance but do not replace Windows Authenticode signing, vulnerability review or user judgment. Draft/release-candidate installers may be unsigned until project-owned signing is available.
