# Release Artifacts

## Current Public Baseline

- Latest tagged version: `v0.1.6`
- App/package version: `0.1.6`
- Local artifact directory: `release/v0.1.6/`
- Download source for users: GitHub Releases latest page

The working tree may contain `Unreleased` fixes on top of `v0.1.6`. Do not describe those changes as a published version until the version is bumped, installers are rebuilt, checksums are regenerated, and a new tag is created.

## Local `release/` Layout

The local `release/` directory is ignored by git and stores generated installers only:

- `release/v0.1.6/` is the latest public package set.
- `release/v0.1.5/` is a historical package set kept for comparison or rollback.
- `release/v0.1.2/`, `release/v0.1.3/`, and `release/v0.1.4/` are historical package sets kept for comparison or rollback.
- `dist/`, `src-tauri/target/`, `node_modules/`, and `release/` must stay out of commits.

When handing a build to users, prefer `release/v0.1.6/` unless a newer version has been intentionally prepared and tagged.

## Version Sync Check

Run this before publishing:

```powershell
Select-String -Path package.json,src-tauri/tauri.conf.json,src-tauri/Cargo.toml,src/lib/app-config.js -Pattern '"version"|version =|appVersion'
Select-String -Path CHANGELOG.md,docs/ROADMAP.md,docs/RELEASES.md -Pattern '^## |Latest tagged version|当前公开版本线|current release line'
```

After `npm run build` or `npm run tauri:build`, confirm generated artifacts remain ignored unless the release process explicitly asks for an external copy.
