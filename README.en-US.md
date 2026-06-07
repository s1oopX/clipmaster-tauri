# ClipMaster

[简体中文](./README.md) | [English](./README.en-US.md)

ClipMaster is a lightweight clipboard manager for Windows desktop. Built with Tauri 2, Rust, Svelte 5, Vite 8, and SQLite, it helps turn text snippets, images, screenshots, and frequently reused content into a searchable local history that can be pinned, starred, deleted, and restored.

The project is currently in an early release stage. It is suitable as a personal productivity tool, a Tauri desktop application reference, and a practical example of a local Rust + Svelte app architecture.

## Download

The latest Windows installers are published on [GitHub Releases](https://github.com/s1oopX/clipmaster-tauri/releases/latest).

Current release artifacts:

- `ClipMaster_0.1.0_x64-setup.exe`: recommended Windows NSIS installer
- `ClipMaster_0.1.0_x64_en-US.msi`: Windows MSI installer

When building from source, installers are generated at:

```text
src-tauri/target/release/bundle/nsis/
src-tauri/target/release/bundle/msi/
```

## Features

- Tracks text and image clipboard content
- Stores history locally with SQLite, without a cloud service
- Supports search, sessions, starring, pinning, and deletion
- Saves image files, renders thumbnails, and copies images back to the clipboard
- Supports region screenshots, screenshot hotkeys, and pinned image windows
- Supports system tray behavior: closing the main window hides it to the tray
- Supports custom cleanup settings and image file lifecycle management
- Supports legacy data directory migration to reduce upgrade data-loss risk

## Tech Stack

- Tauri 2
- Rust 2021
- Svelte 5
- Vite 8
- SQLite / rusqlite
- Vitest + Svelte Testing Library

## Quick Start

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

The default development port is `5174`. The in-app Settings / Port panel can check whether the port is occupied and switch to another port. Local development configuration is written to the ignored `.clipmaster-dev.json` file.

### Build Production Installers

```powershell
npm run tauri:build
```

The build produces the desktop executable, MSI installer, and NSIS installer. The Rust/Tauri `src-tauri/target` build cache can become large; it can be removed when old build artifacts are no longer needed, and it will be regenerated on the next build.

## Validation

Recommended release checks:

```powershell
npm test
npm run build
cd src-tauri
cargo fmt --check
cargo check
cargo test
cd ..
npm run tauri:build
```

As of 2026-06-07, the project has been verified to generate:

```text
src-tauri/target/release/clipmaster.exe
src-tauri/target/release/bundle/msi/ClipMaster_0.1.0_x64_en-US.msi
src-tauri/target/release/bundle/nsis/ClipMaster_0.1.0_x64-setup.exe
```

## Project Structure

```text
src/                 Svelte frontend
src/components/      Frontend components
src/lib/             Frontend API, configuration, and UI helpers
src-tauri/src/       Rust backend, database, clipboard, tray, and commands
src-tauri/icons/     Application icons
docs/                Architecture, API, database, workflow, and troubleshooting docs
```

## Documentation

- [Next Steps](./docs/NEXT_STEPS.md)
- [Roadmap](./docs/ROADMAP.md)
- [Architecture](./docs/ARCHITECTURE.md)
- [API](./docs/API.md)
- [Database](./docs/DATABASE.md)
- [Workflow](./docs/WORKFLOW.md)
- [Troubleshooting](./docs/TROUBLESHOOTING.md)
- [Changelog](./CHANGELOG.md)

## Contributing

Issues, suggestions, and pull requests are welcome. Before contributing, review the [workflow](./docs/WORKFLOW.md) and [next steps](./docs/NEXT_STEPS.md), then add the relevant frontend, backend, or packaging validation for the change.

## License

This project is open-sourced under the [MIT License](./LICENSE).
