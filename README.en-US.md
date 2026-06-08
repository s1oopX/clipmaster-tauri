<div align="center">

<img src="./src-tauri/icons/icon.png" alt="ClipMaster icon" width="96" height="96">

# ClipMaster

A local-first Windows clipboard manager for recovering, organizing, and reusing copied text, images, and screenshots.

[简体中文](./README.md) · [Latest Release](https://github.com/s1oopX/clipmaster-tauri/releases/latest) · [Roadmap](./docs/ROADMAP.md)

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows-0078D4.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg)
![Rust](https://img.shields.io/badge/Rust-2021-B7410E.svg)
![Svelte](https://img.shields.io/badge/Svelte-5-FF3E00.svg)

</div>

## Why ClipMaster

Clipboard history is one of the easiest parts of a workflow to lose: a command you copied a minute ago, a temporary screenshot, or a snippet you paste repeatedly can disappear as soon as the next item replaces it. ClipMaster keeps that content on your machine so it can be searched, starred, pinned, and copied back to the clipboard.

It is not a cloud clipboard or an account-based sync product. The current goal is intentionally narrow: a lightweight, understandable, and controllable local clipboard history for Windows desktop.

## Download

Installers are available from [GitHub Releases](https://github.com/s1oopX/clipmaster-tauri/releases/latest).

| File | Purpose |
| --- | --- |
| [`ClipMaster_0.1.1_x64-setup.exe`](https://github.com/s1oopX/clipmaster-tauri/releases/download/v0.1.1/ClipMaster_0.1.1_x64-setup.exe) | Recommended installer for most Windows users |
| [`ClipMaster_0.1.1_x64_en-US.msi`](https://github.com/s1oopX/clipmaster-tauri/releases/download/v0.1.1/ClipMaster_0.1.1_x64_en-US.msi) | MSI installer for traditional deployment flows |

The current build is not code-signed yet, so Windows SmartScreen may show a warning. Download installers from this repository's Release page and verify files with the `SHA256SUMS.txt` file attached to the Release.

## Features

| Feature | Details |
| --- | --- |
| Text and image history | Automatically tracks text and image clipboard content |
| Search and filtering | Finds history items, sessions, and frequently reused snippets |
| Star and pin | Keeps important items easy to find |
| Image workflow | Saves images, generates thumbnails, previews them, and copies them back |
| Screenshot helper | Supports region screenshots, screenshot hotkeys, and pinned image windows |
| System tray | Closing the main window hides it to the tray, with restore and quit actions |
| Cleanup settings | Supports count-based, age-based, image-file lifecycle cleanup, and full history clearing |
| Data migration | Migrates legacy data directories to reduce upgrade data-loss risk |

## Privacy and Data

ClipMaster keeps the local machine as the default data boundary:

- Clipboard history is stored in a local SQLite database.
- Images and thumbnails are stored in the local app data directory.
- The current version does not provide cloud sync, account login, or remote telemetry.
- Clipboard content can include passwords, tokens, and personal data, so pause monitoring before copying sensitive content or use the full history clear action in Advanced settings.

## Project Status

ClipMaster 0.1.1 is an early usable release. It covers the core clipboard, image, screenshot, tray, migration, and packaging workflows. It is suitable for personal daily testing and as a reference project for Tauri 2 + Rust + Svelte desktop application architecture.

The next priorities are session navigation, virtual scrolling, more global shortcuts, and further frontend state separation. See the [Roadmap](./docs/ROADMAP.md) and [Next Steps](./docs/NEXT_STEPS.md).

## Tech Stack

- Tauri 2
- Rust 2021
- Svelte 5
- Vite 8
- SQLite / rusqlite
- Vitest + Svelte Testing Library

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

### Build Installers

```powershell
npm run tauri:build
```

Build artifacts are generated at:

```text
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

`src-tauri/target` is the Rust/Tauri build cache and can become large. Removing it does not affect the source code; the next build will regenerate it, although the first rebuild will take longer.

## Commands

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
src/                 Svelte frontend entry
src/components/      Frontend components
src/lib/             Frontend API, configuration, and UI helpers
src-tauri/src/       Rust backend, database, clipboard, tray, and commands
src-tauri/icons/     Application icons
docs/                Architecture, API, database, workflow, and troubleshooting docs
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

## License

ClipMaster is open-sourced under the [MIT License](./LICENSE).
