# Security Policy

ClipMaster is a local-first clipboard manager. Security and privacy reports are taken seriously because clipboard history can contain sensitive text, images, passwords, tokens, and screenshots.

## Supported Versions

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |

## Reporting a Vulnerability

If the issue may expose private clipboard content, local files, credentials, tokens, or unsafe update/install behavior, please avoid posting sensitive details in a public issue.

Preferred reporting path:

1. Open a private GitHub security advisory for this repository if available.
2. If private advisory reporting is not available, open a minimal public issue that says a security report exists, without including secrets or exploit details.

Please include:

- ClipMaster version
- Windows version
- Whether the app was installed from GitHub Releases or built from source
- Clear reproduction steps using non-sensitive sample data
- Expected and actual behavior

## Current Security Notes

- Installers are not code-signed yet, so Windows SmartScreen may warn.
- Release assets include SHA256 checksums for manual verification.
- Clipboard history is stored locally; there is currently no cloud sync or telemetry.
- Application blacklists and automatic sensitive-content detection are not implemented yet.
