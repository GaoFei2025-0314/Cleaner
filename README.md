# Cleaner

Cleaner is a Windows-only local desktop app plan for safely scanning and cleaning disks, with C drive cleanup as the primary scenario.

## Version

Current planned baseline: **V0.1**.

V0.1 is the safe baseline:

- Normal user mode by default.
- C drive only.
- Two result views: by risk and by source.
- Recommended items can be preselected.
- Optional and high-risk items require manual selection.
- High-risk items require second confirmation.
- Not-cleanable items cannot be selected.
- WeChat and QQ root data directories are not cleanable in V0.1.
- Administrator cleanup is described only; elevated execution is deferred to V0.2.

## Current Outputs

- Product/spec design: `docs/superpowers/specs/2026-06-11-c-drive-cleaner-design.md`
- Code-level implementation plan: `docs/superpowers/plans/2026-06-11-c-drive-cleaner-implementation.md`
- Chinese implementation plan: `docs/superpowers/plans/2026-06-11-c-drive-cleaner-implementation.zh-CN.md`

## Current Status

V0.1 implementation is in progress on branch `v0.1-implementation`.

## Development

Install dependencies:

```powershell
npm install
```

Run frontend tests:

```powershell
npm test
```

Run frontend production build:

```powershell
npm run build
```

Run Rust tests:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Run browser preview with mock data:

```powershell
npm run dev
```

Run Tauri desktop app:

```powershell
npm run tauri:dev
```

Build installer:

```powershell
npm run tauri:build
```

## Safety Boundary

The frontend must not construct cleanup paths or decide filesystem safety. It can only display backend scan results and submit selected item IDs. Rust backend modules own rule resolution, path allowlist checks, configuration reference checks, process usage checks, and deletion execution.
