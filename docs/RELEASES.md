# Release Artifacts

## Publishing Flow (CI-first)

自 v0.1.7 之后，发布资产以 CI 构建为准：

1. 版本号同步 + CHANGELOG 定版，提交并推送 `v*` tag。
2. `Release Build` 工作流构建安装包与 `SHA256SUMS.txt`，并自动挂载到该 tag 的 GitHub Release（不存在则创建**草稿**）。
3. 人工核对草稿的资产与校验和，补充 Release 说明后发布。
4. 本地 `release/vX.Y.Z/` 目录仅作为对照留档，不再作为发布来源。

配置了 SignPath 凭据时，`sign-installers` job 产出已签名产物（见 `SIGNING.md`），发布前用已签名版本替换草稿资产。

## Current Public Baseline

- Latest tagged version: `v0.1.7`
- App/package version: `0.1.7`
- Local artifact directory: `release/v0.1.7/`
- Download source for users: GitHub Releases latest page

The working tree may contain `Unreleased` fixes on top of `v0.1.7`. Do not describe those changes as a published version until the version is bumped, installers are rebuilt, checksums are regenerated, and a new tag is created.

## Local `release/` Layout

The local `release/` directory is ignored by git and stores generated installers only:

- `release/v0.1.7/` is the latest public package set.
- `release/v0.1.6/` and `release/v0.1.5/` are historical package sets kept for comparison or rollback.
- `release/v0.1.2/`, `release/v0.1.3/`, and `release/v0.1.4/` are historical package sets kept for comparison or rollback.
- `dist/`, `src-tauri/target/`, `node_modules/`, and `release/` must stay out of commits.

When handing a build to users, prefer `release/v0.1.7/` unless a newer version has been intentionally prepared and tagged.

## Version Sync Check

Run this before publishing:

```powershell
Select-String -Path package.json,src-tauri/tauri.conf.json,src-tauri/Cargo.toml,src/lib/app-config.js -Pattern '"version"|version =|appVersion'
Select-String -Path CHANGELOG.md,docs/ROADMAP.md,docs/RELEASES.md -Pattern '^## |Latest tagged version|当前公开版本线|current release line'
```

After `npm run build` or `npm run tauri:build`, confirm generated artifacts remain ignored unless the release process explicitly asks for an external copy.
