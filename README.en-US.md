<div align="center">

<img src="./src-tauri/icons/icon.png" alt="ClipMaster icon" width="96" height="96">

# ClipMaster

A local-first Windows clipboard manager for recording, searching, and reusing text, images, and screenshots.

[简体中文](./README.md) · [Latest Release](https://github.com/s1oopX/clipmaster-tauri/releases/latest) · [Roadmap](./docs/ROADMAP.md) · [Security](./SECURITY.md)

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-0078D4.svg)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB.svg)
![Rust](https://img.shields.io/badge/Rust-2021-B7410E.svg)
![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00.svg)

</div>

## Overview

ClipMaster is a local Windows desktop utility for clipboard history, image reuse, and screenshot workflows. It stores copied text, copied images, and captured screenshots on the local machine, then makes them searchable, reusable, pinnable, annotatable, and clearable.

The project is not a cloud clipboard, account-based sync service, or telemetry product. Its goal is to provide a lightweight and controllable local clipboard workspace for personal workflows, development, documentation, support, and repeated image reuse.

## Project Status

The current release line is `0.1.1`. The project already covers everyday clipboard history, image history, region screenshots, frozen-screen selection, basic annotation, pinned desktop images, system tray behavior, settings, cleanup policies, data migration, and Windows packaging.

Planned improvements include virtual scrolling, additional global shortcuts, OCR, scrolling screenshots, backup and restore, automatic updates, and code signing. See the [Roadmap](./docs/ROADMAP.md) for details.

## Download and Installation

Installers are published through [GitHub Releases](https://github.com/s1oopX/clipmaster-tauri/releases/latest).

| File | Use case |
| --- | --- |
| `ClipMaster_0.1.1_x64-setup.exe` | Recommended installer for most Windows users |
| `ClipMaster_0.1.1_x64_en-US.msi` | MSI package for traditional deployment or environments that require MSI |
| `SHA256SUMS.txt` | Release artifact checksums |

Current builds are not code-signed yet, so Windows SmartScreen may show a warning. Download installers only from this repository's Release page and verify them with the SHA256 file attached to the Release.

## Features

| Area | Capabilities |
| --- | --- |
| Clipboard history | Records text and image clipboard content and copies items back to the clipboard |
| Search and filtering | Supports content search, type filters, date filters, and session history |
| Favorites and pins | Keeps important records easier to find |
| Image workflow | Saves images, generates thumbnails, previews images, copies images, and pins images to the desktop |
| Screenshot workflow | Region capture, frozen-screen selection, automatic clipboard copy, history saving, reselecting, rectangle/arrow/pen annotation |
| Desktop image pinning | Opens images as always-on-top reference windows |
| System tray | Closing the main window hides it to the tray, with restore and quit actions |
| Settings | Supports screenshot hotkeys, capture delay, retention count, time zone, language, and optional autostart |
| Cleanup | Supports count-based cleanup, age-based cleanup, image-file lifecycle cleanup, and full history clearing |
| Migration | Includes legacy data directory migration and database schema migrations |

## Screenshot Workflow

ClipMaster is designed for "capture and use immediately" screenshot workflows:

- Starting a screenshot first captures the active screen and opens a frozen image for selection.
- The selection can be moved, resized with eight handles, and nudged by 1 pixel with arrow keys.
- Confirming a screenshot saves it to history and writes it to the system clipboard, so it can be pasted immediately with `Ctrl+V`.
- Rectangle, arrow, and pen annotations are included in the final image.
- Users can reselect the capture area or pin the result directly to the desktop.

## Privacy and Data

ClipMaster keeps the local machine as its default trust boundary:

- Clipboard history is stored in a local SQLite database.
- Images, thumbnails, and screenshots are stored in the local app data directory.
- The current version does not upload clipboard content, provide cloud sync, or include remote telemetry.
- Clipboard content may include passwords, tokens, customer data, or sensitive screenshots. Pause monitoring before copying sensitive content, or use the full history clearing action in Advanced settings.

Default data directory:

```text
%APPDATA%/com.clipmaster.desktop/
```

See [Privacy](./docs/PRIVACY.md) and [Database](./docs/DATABASE.md) for more details.

## Tech Stack

- Tauri 2
- Rust 2021
- Svelte 5
- Vite 8
- SQLite / rusqlite
- Vitest / Svelte Testing Library
- screenshots / arboard / image

## Local Development

### Requirements

- Windows 10/11
- Node.js 18 or later
- npm
- Rust stable
- Visual Studio Build Tools with the C++ workload

### Install Dependencies

```powershell
npm install
```

### Start Development Mode

```powershell
npm run tauri:dev
```

The default development port is `5174`. The in-app settings panel can check port availability and switch ports. Local development configuration is written to `.clipmaster-dev.json`, which is intentionally ignored by Git.

### Build the Windows App and Installers

```powershell
npm run tauri:build
```

Build artifacts are generated at:

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

`node_modules`, `dist`, and `src-tauri/target` are generated development or build artifacts and should not be committed to the repository.

## Common Commands

| Command | Description |
| --- | --- |
| `npm run tauri:dev` | Start the Tauri development window |
| `npm test` | Run frontend tests |
| `npm run build` | Build frontend assets |
| `npm run tauri:build` | Build the Windows desktop app and installers |
| `cargo fmt --check` | Check Rust formatting |
| `cargo check` | Check Rust compilation |
| `cargo test` | Run Rust unit tests |

## Project Structure

```text
src/                 Svelte frontend entry and page logic
src/components/      Frontend components
src/lib/             Frontend API, configuration, and UI helpers
src-tauri/src/       Rust backend, database, clipboard, tray, commands, and settings
src-tauri/icons/     Application icons
docs/                Architecture, API, database, workflow, roadmap, and troubleshooting docs
public/              Static assets
scripts/             Local development scripts
```

## Documentation

- [Roadmap](./docs/ROADMAP.md)
- [Next Steps](./docs/NEXT_STEPS.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [API](./docs/API.md)
- [Database](./docs/DATABASE.md)
- [Workflow](./docs/WORKFLOW.md)
- [Privacy and Data](./docs/PRIVACY.md)
- [FAQ](./docs/FAQ.md)
- [Security Policy](./SECURITY.md)
- [Troubleshooting](./docs/TROUBLESHOOTING.md)
- [Changelog](./CHANGELOG.md)

## Contributing

Issues, suggestions, and pull requests are welcome. Before making changes, review the [development workflow](./docs/WORKFLOW.md), then add the relevant frontend tests, Rust tests, or packaging checks for the scope of the change.

If a report involves clipboard content, screenshots, tokens, passwords, or other sensitive data, do not paste real content into public issues. For security issues, see the [Security Policy](./SECURITY.md).

## License

ClipMaster is open-sourced under the [MIT License](./LICENSE).
