# C Drive Cleaner V0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build V0.1 of a Windows-only local desktop app that scans the C drive, classifies cleanup candidates by safety and source, requires user confirmation, and deletes only selected items after revalidation.

**Architecture:** Use a Tauri 2 desktop shell with a React + TypeScript wizard UI and a Rust backend for filesystem scanning, safety checks, deletion, administrator-mode commands, and privacy-safe analytics payload construction. The backend owns all filesystem decisions; the frontend only displays backend scan results and sends selected item IDs back for cleanup.

**Tech Stack:** Tauri 2, React, TypeScript, Vite, Rust, Serde, sysinfo, Vitest, Testing Library, Cargo tests, Windows NSIS installer through Tauri bundler.

---

## Scope Check

The product spec contains several subsystems: desktop UI, scan rules, safety checks, cleanup executor, administrator mode, analytics, and packaging. They are coupled by one central domain model, so this plan keeps them in one implementation track while making each task independently testable.

The V0.1 working slice must support:

- Normal-mode scan of user-owned C drive paths.
- Risk and source grouping.
- Configuration reference protection.
- Process usage protection.
- Direct deletion after confirmation.
- High-risk second confirmation.
- Privacy settings and analytics payload sanitization.

V0.1 deliberately does not implement broad root-directory deletion for chat apps and does not execute elevated administrator cleanup. Those are planned for later versions after more granular classifiers and elevation flow tests exist.

Administrator mode and installer packaging come after the normal-mode workflow is passing.

## File Structure

Create this project structure under `H:\临时对话\c-drive-cleaner`.

```text
c-drive-cleaner/
  package.json
  package-lock.json
  index.html
  vite.config.ts
  tsconfig.json
  tsconfig.node.json
  vitest.config.ts
  src/
    main.tsx
    App.tsx
    styles/theme.css
    domain/models.ts
    domain/selection.ts
    domain/grouping.ts
    services/tauriApi.ts
    services/mockReport.ts
    components/AppShell.tsx
    components/StepIndicator.tsx
    components/WelcomeStep.tsx
    components/ScanStep.tsx
    components/SuggestionsStep.tsx
    components/ConfirmStep.tsx
    components/CleanStep.tsx
    components/ResultStep.tsx
    components/PrivacyNotice.tsx
    components/RiskBadge.tsx
    components/CleanupItemRow.tsx
    __tests__/selection.test.ts
    __tests__/grouping.test.ts
  src-tauri/
    Cargo.toml
    tauri.conf.json
    build.rs
    src/
      main.rs
      lib.rs
      models.rs
      drive.rs
      paths.rs
      size.rs
      rules.rs
      scan.rs
      config_refs.rs
      processes.rs
      cleanup.rs
      admin.rs
      analytics.rs
      errors.rs
      fixtures.rs
      tests/
        rule_registry_tests.rs
        config_refs_tests.rs
        cleanup_tests.rs
        analytics_tests.rs
```

Responsibility boundaries:

- `src-tauri/src/models.rs`: DTOs shared with the frontend.
- `src-tauri/src/rules.rs`: bundled cleanup rule definitions.
- `src-tauri/src/scan.rs`: transforms rules into scan items.
- `src-tauri/src/config_refs.rs`: detects path references in known config files.
- `src-tauri/src/processes.rs`: detects running processes that point into candidate paths.
- `src-tauri/src/cleanup.rs`: revalidates and deletes selected items.
- `src-tauri/src/admin.rs`: user-initiated elevated commands for lightweight system cleanup.
- `src-tauri/src/analytics.rs`: constructs privacy-safe analytics events.
- `src/domain/*.ts`: frontend-only grouping and selection logic.
- `src/components/*.tsx`: wizard UI.

## Task 1: Initialize The Desktop App Skeleton

**Files:**

- Create: `package.json`
- Create: `index.html`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `vitest.config.ts`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: Initialize git in the project folder**

Run from `H:\临时对话\c-drive-cleaner`:

```powershell
git init
git status --short
```

Expected: `git status --short` prints the existing `README.md` and `docs/` files as untracked.

- [ ] **Step 2: Create the frontend package manifest**

Create `package.json`:

```json
{
  "name": "c-drive-cleaner",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "clsx": "^2.1.1",
    "lucide-react": "^0.468.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0",
    "@testing-library/jest-dom": "^6.6.3",
    "@testing-library/react": "^16.1.0",
    "@types/react": "^18.3.12",
    "@types/react-dom": "^18.3.1",
    "@vitejs/plugin-react": "^4.3.4",
    "jsdom": "^25.0.1",
    "typescript": "^5.6.3",
    "vite": "^5.4.11",
    "vitest": "^2.1.5"
  }
}
```

- [ ] **Step 3: Create Vite and TypeScript config**

Create `index.html`:

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>C Drive Cleaner</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Create `vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
```

Create `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["DOM", "DOM.Iterable", "ES2020"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx"
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

Create `tsconfig.node.json`:

```json
{
  "compilerOptions": {
    "composite": true,
    "module": "ESNext",
    "moduleResolution": "Node",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts", "vitest.config.ts"]
}
```

Create `vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: [],
  },
});
```

- [ ] **Step 4: Create minimal React entry files**

Create `src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/theme.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

Create `src/App.tsx`:

```tsx
export default function App() {
  return (
    <main className="app-root">
      <section className="empty-shell">
        <p className="eyebrow">C Drive Cleaner</p>
        <h1>安全清理 C 盘</h1>
        <p>项目骨架已启动。后续任务会接入扫描、建议、确认和清理流程。</p>
      </section>
    </main>
  );
}
```

Create `src/styles/theme.css`:

```css
:root {
  color: #172026;
  background: #f4f8f7;
  font-family: "Microsoft YaHei UI", "Segoe UI", sans-serif;
}

body {
  margin: 0;
  min-width: 1024px;
  min-height: 720px;
}

button,
input {
  font: inherit;
}

.app-root {
  min-height: 100vh;
  display: grid;
  place-items: center;
}

.empty-shell {
  width: min(760px, calc(100vw - 96px));
  border: 1px solid #d6e4e2;
  background: #ffffff;
  border-radius: 8px;
  padding: 40px;
  box-shadow: 0 18px 50px rgba(30, 66, 68, 0.12);
}

.eyebrow {
  margin: 0 0 8px;
  color: #127a75;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0;
  text-transform: uppercase;
}

h1 {
  margin: 0 0 12px;
  font-size: 34px;
  line-height: 1.2;
}

p {
  line-height: 1.7;
}
```

- [ ] **Step 5: Create Tauri Rust skeleton**

Create `src-tauri/Cargo.toml`:

```toml
[package]
name = "c-drive-cleaner"
version = "0.1.0"
description = "Safe local Windows C drive cleaner"
authors = ["C Drive Cleaner"]
edition = "2021"

[lib]
name = "c_drive_cleaner"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2.0.0", features = [] }

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tauri = { version = "2.0.0", features = [] }
thiserror = "2.0"
sysinfo = "0.33"
walkdir = "2.5"
time = { version = "0.3", features = ["formatting"] }
```

Create `src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "C Drive Cleaner",
  "version": "0.1.0",
  "identifier": "app.cdrivecleaner.desktop",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "C Drive Cleaner",
        "width": 1180,
        "height": 760,
        "minWidth": 1024,
        "minHeight": 720,
        "resizable": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "publisher": "C Drive Cleaner",
    "shortDescription": "Safe Windows C drive cleaner",
    "longDescription": "A local Windows desktop app that scans C drive cleanup candidates and requires explicit confirmation before deletion."
  }
}
```

Create `src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build();
}
```

Create `src-tauri/src/main.rs`:

```rust
fn main() {
    c_drive_cleaner::run();
}
```

Create `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 6: Install dependencies and verify skeleton**

Run:

```powershell
npm install
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected:

- `npm install` creates `package-lock.json`.
- `npm run build` exits 0 and creates `dist/`.
- `cargo test` exits 0.

- [ ] **Step 7: Commit skeleton**

Run:

```powershell
git add .
git commit -m "chore: initialize tauri desktop app"
```

Expected: commit succeeds.

## Task 2: Define Shared Domain Models

**Files:**

- Create: `src/domain/models.ts`
- Create: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add TypeScript domain models**

Create `src/domain/models.ts`:

```ts
export type RiskLevel = "recommended" | "optional" | "highRisk" | "notCleanable";

export type SourceCategory =
  | "system"
  | "commonSoftware"
  | "wechat"
  | "qq"
  | "workChat"
  | "cloudDrive"
  | "installersOldVersions"
  | "otherLarge";

export type CleanupAction =
  | "directDelete"
  | "requiresAdmin"
  | "explainOnly"
  | "blockedByProcess"
  | "blockedByConfigReference";

export interface DriveSummary {
  drive: "C:";
  totalBytes: number;
  freeBytes: number;
}

export interface ScanItem {
  id: string;
  title: string;
  description: string;
  sourceCategory: SourceCategory;
  riskLevel: RiskLevel;
  cleanupAction: CleanupAction;
  estimatedBytes: number;
  defaultSelected: boolean;
  userVisiblePathHint: string;
  technicalPath?: string;
  reasons: string[];
  warnings: string[];
}

export interface ScanReport {
  driveSummary: DriveSummary;
  items: ScanItem[];
  partial: boolean;
  scanStartedAt: string;
  scanFinishedAt: string;
}

export interface CleanupSelection {
  selectedItemIds: string[];
  highRiskConfirmed: boolean;
  requestAdminMode: boolean;
}

export interface CleanupItemResult {
  itemId: string;
  status: "deleted" | "skipped" | "failed";
  freedBytes: number;
  message: string;
}

export interface CleanupResult {
  results: CleanupItemResult[];
  totalFreedBytes: number;
  finishedAt: string;
}
```

- [ ] **Step 2: Add Rust models**

Create `src-tauri/src/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    Recommended,
    Optional,
    HighRisk,
    NotCleanable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourceCategory {
    System,
    CommonSoftware,
    Wechat,
    Qq,
    WorkChat,
    CloudDrive,
    InstallersOldVersions,
    OtherLarge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CleanupAction {
    DirectDelete,
    RequiresAdmin,
    ExplainOnly,
    BlockedByProcess,
    BlockedByConfigReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriveSummary {
    pub drive: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source_category: SourceCategory,
    pub risk_level: RiskLevel,
    pub cleanup_action: CleanupAction,
    pub estimated_bytes: u64,
    pub default_selected: bool,
    pub user_visible_path_hint: String,
    pub technical_path: Option<String>,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub drive_summary: DriveSummary,
    pub items: Vec<ScanItem>,
    pub partial: bool,
    pub scan_started_at: String,
    pub scan_finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupSelection {
    pub selected_item_ids: Vec<String>,
    pub high_risk_confirmed: bool,
    pub request_admin_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItemResult {
    pub item_id: String,
    pub status: String,
    pub freed_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub results: Vec<CleanupItemResult>,
    pub total_freed_bytes: u64,
    pub finished_at: String,
}
```

- [ ] **Step 3: Wire Rust module**

Modify `src-tauri/src/lib.rs`:

```rust
mod models;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 4: Verify model code compiles**

Run:

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: both commands exit 0.

- [ ] **Step 5: Commit models**

Run:

```powershell
git add src/domain/models.ts src-tauri/src/models.rs src-tauri/src/lib.rs
git commit -m "feat: add shared cleanup domain models"
```

Expected: commit succeeds.

## Task 3: Implement Frontend Selection And Grouping Logic

**Files:**

- Create: `src/domain/selection.ts`
- Create: `src/domain/grouping.ts`
- Create: `src/__tests__/selection.test.ts`
- Create: `src/__tests__/grouping.test.ts`

- [ ] **Step 1: Write selection tests**

Create `src/__tests__/selection.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { ScanItem } from "../domain/models";
import { buildDefaultSelection, toggleSelection, requiresHighRiskConfirmation } from "../domain/selection";

const items: ScanItem[] = [
  {
    id: "temp",
    title: "用户临时文件",
    description: "临时文件",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "directDelete",
    estimatedBytes: 100,
    defaultSelected: true,
    userVisiblePathHint: "用户临时目录",
    reasons: ["推荐清理"],
    warnings: [],
  },
  {
    id: "windows-temp",
    title: "Windows 临时文件",
    description: "需要管理员权限",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "requiresAdmin",
    estimatedBytes: 150,
    defaultSelected: true,
    userVisiblePathHint: "Windows 临时目录",
    reasons: ["V0.1 仅展示管理员清理能力"],
    warnings: ["当前版本不会执行提权清理"],
  },
  {
    id: "wechat-video",
    title: "微信视频",
    description: "聊天视频",
    sourceCategory: "wechat",
    riskLevel: "highRisk",
    cleanupAction: "directDelete",
    estimatedBytes: 200,
    defaultSelected: false,
    userVisiblePathHint: "微信数据目录",
    reasons: ["用户手动确认后可清理"],
    warnings: ["可能删除聊天视频"],
  },
  {
    id: "config-ref",
    title: "工具运行目录",
    description: "被配置引用",
    sourceCategory: "installersOldVersions",
    riskLevel: "notCleanable",
    cleanupAction: "blockedByConfigReference",
    estimatedBytes: 300,
    defaultSelected: false,
    userVisiblePathHint: "工具目录",
    reasons: ["被配置引用"],
    warnings: [],
  },
];

describe("selection", () => {
  it("selects only default selected cleanable items", () => {
    expect(buildDefaultSelection(items)).toEqual(["temp"]);
  });

  it("does not select not cleanable items", () => {
    expect(toggleSelection(["temp"], items[3], true)).toEqual(["temp"]);
  });

  it("does not default select or toggle admin-required items in V0.1", () => {
    expect(buildDefaultSelection(items)).toEqual(["temp"]);
    expect(toggleSelection(["temp"], items[1], true)).toEqual(["temp"]);
  });

  it("allows users to select high risk cleanable items", () => {
    expect(toggleSelection(["temp"], items[2], true)).toEqual(["temp", "wechat-video"]);
  });

  it("requires confirmation when a high risk item is selected", () => {
    expect(requiresHighRiskConfirmation(["temp", "wechat-video"], items)).toBe(true);
  });
});
```

- [ ] **Step 2: Write grouping tests**

Create `src/__tests__/grouping.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { ScanItem } from "../domain/models";
import { groupByRisk, groupBySource } from "../domain/grouping";

const items: ScanItem[] = [
  {
    id: "a",
    title: "系统临时文件",
    description: "临时文件",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "directDelete",
    estimatedBytes: 1,
    defaultSelected: true,
    userVisiblePathHint: "系统",
    reasons: [],
    warnings: [],
  },
  {
    id: "b",
    title: "微信图片",
    description: "图片",
    sourceCategory: "wechat",
    riskLevel: "highRisk",
    cleanupAction: "directDelete",
    estimatedBytes: 2,
    defaultSelected: false,
    userVisiblePathHint: "微信",
    reasons: [],
    warnings: [],
  },
];

describe("grouping", () => {
  it("groups items by risk", () => {
    const grouped = groupByRisk(items);
    expect(grouped.recommended.map((item) => item.id)).toEqual(["a"]);
    expect(grouped.highRisk.map((item) => item.id)).toEqual(["b"]);
  });

  it("groups items by source", () => {
    const grouped = groupBySource(items);
    expect(grouped.system.map((item) => item.id)).toEqual(["a"]);
    expect(grouped.wechat.map((item) => item.id)).toEqual(["b"]);
  });
});
```

- [ ] **Step 3: Run tests and verify they fail**

Run:

```powershell
npm test
```

Expected: tests fail because `src/domain/selection.ts` and `src/domain/grouping.ts` do not exist.

- [ ] **Step 4: Implement selection logic**

Create `src/domain/selection.ts`:

```ts
import type { ScanItem } from "./models";

function isSelectable(item: ScanItem): boolean {
  return item.riskLevel !== "notCleanable" && item.cleanupAction === "directDelete";
}

export function buildDefaultSelection(items: ScanItem[]): string[] {
  return items.filter((item) => item.defaultSelected && isSelectable(item)).map((item) => item.id);
}

export function toggleSelection(current: string[], item: ScanItem, checked: boolean): string[] {
  if (!isSelectable(item)) {
    return current;
  }

  const currentSet = new Set(current);
  if (checked) {
    currentSet.add(item.id);
  } else {
    currentSet.delete(item.id);
  }
  return Array.from(currentSet);
}

export function requiresHighRiskConfirmation(selectedIds: string[], items: ScanItem[]): boolean {
  const selected = new Set(selectedIds);
  return items.some((item) => selected.has(item.id) && item.riskLevel === "highRisk");
}

export function estimateSelectedBytes(selectedIds: string[], items: ScanItem[]): number {
  const selected = new Set(selectedIds);
  return items.reduce((total, item) => (selected.has(item.id) ? total + item.estimatedBytes : total), 0);
}
```

- [ ] **Step 5: Implement grouping logic**

Create `src/domain/grouping.ts`:

```ts
import type { RiskLevel, ScanItem, SourceCategory } from "./models";

export type RiskGroups = Record<RiskLevel, ScanItem[]>;
export type SourceGroups = Record<SourceCategory, ScanItem[]>;

export function groupByRisk(items: ScanItem[]): RiskGroups {
  return {
    recommended: items.filter((item) => item.riskLevel === "recommended"),
    optional: items.filter((item) => item.riskLevel === "optional"),
    highRisk: items.filter((item) => item.riskLevel === "highRisk"),
    notCleanable: items.filter((item) => item.riskLevel === "notCleanable"),
  };
}

export function groupBySource(items: ScanItem[]): SourceGroups {
  return {
    system: items.filter((item) => item.sourceCategory === "system"),
    commonSoftware: items.filter((item) => item.sourceCategory === "commonSoftware"),
    wechat: items.filter((item) => item.sourceCategory === "wechat"),
    qq: items.filter((item) => item.sourceCategory === "qq"),
    workChat: items.filter((item) => item.sourceCategory === "workChat"),
    cloudDrive: items.filter((item) => item.sourceCategory === "cloudDrive"),
    installersOldVersions: items.filter((item) => item.sourceCategory === "installersOldVersions"),
    otherLarge: items.filter((item) => item.sourceCategory === "otherLarge"),
  };
}
```

- [ ] **Step 6: Run tests and verify they pass**

Run:

```powershell
npm test
```

Expected: selection and grouping tests pass.

- [ ] **Step 7: Commit frontend domain logic**

Run:

```powershell
git add src/domain src/__tests__
git commit -m "feat: add cleanup selection and grouping logic"
```

Expected: commit succeeds.

## Task 4: Implement Rust Rule Registry

**Files:**

- Create: `src-tauri/src/rules.rs`
- Create: `src-tauri/src/fixtures.rs`
- Create: `src-tauri/tests/rule_registry_tests.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write rule registry tests**

Create `src-tauri/tests/rule_registry_tests.rs`:

```rust
use c_drive_cleaner::rules::{builtin_rules, RuleScope};
use c_drive_cleaner::models::{CleanupAction, RiskLevel, SourceCategory};

#[test]
fn builtin_rules_include_user_temp_as_recommended() {
    let rules = builtin_rules();
    let user_temp = rules.iter().find(|rule| rule.id == "user-temp").expect("user-temp rule");

    assert_eq!(user_temp.title, "用户临时文件");
    assert_eq!(user_temp.risk_level, RiskLevel::Recommended);
    assert_eq!(user_temp.cleanup_action, CleanupAction::DirectDelete);
    assert_eq!(user_temp.source_category, SourceCategory::System);
    assert!(matches!(user_temp.scope, RuleScope::UserLocalAppDataRelative(_)));
}

#[test]
fn builtin_rules_never_default_select_high_risk_items() {
    for rule in builtin_rules() {
        if rule.risk_level == RiskLevel::HighRisk {
            assert!(!rule.default_selected, "high-risk rule selected by default: {}", rule.id);
        }
    }
}

#[test]
fn not_cleanable_rules_are_not_default_selected() {
    for rule in builtin_rules() {
        if rule.risk_level == RiskLevel::NotCleanable {
            assert!(!rule.default_selected, "not-cleanable rule selected by default: {}", rule.id);
        }
    }
}

#[test]
fn admin_required_rules_are_not_default_selected_in_v01() {
    for rule in builtin_rules() {
        if rule.cleanup_action == CleanupAction::RequiresAdmin {
            assert!(!rule.default_selected, "admin-required rule selected by default: {}", rule.id);
        }
    }
}
```

- [ ] **Step 2: Run Rust tests and verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml rule_registry -- --nocapture
```

Expected: tests fail because `rules` is not exported.

- [ ] **Step 3: Implement rule registry**

Create `src-tauri/src/rules.rs`:

```rust
use crate::models::{CleanupAction, RiskLevel, SourceCategory};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleScope {
    UserLocalAppDataRelative(&'static str),
    UserProfileRelative(&'static str),
    WindowsRelative(&'static str),
    Absolute(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupRule {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub source_category: SourceCategory,
    pub risk_level: RiskLevel,
    pub cleanup_action: CleanupAction,
    pub default_selected: bool,
    pub scope: RuleScope,
    pub delete_contents_only: bool,
    pub min_age_minutes: u64,
}

pub fn builtin_rules() -> Vec<CleanupRule> {
    vec![
        CleanupRule {
            id: "user-temp",
            title: "用户临时文件",
            description: "软件运行时留下的临时材料，通常可以安全删除。",
            source_category: SourceCategory::System,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::DirectDelete,
            default_selected: true,
            scope: RuleScope::UserLocalAppDataRelative("Temp"),
            delete_contents_only: true,
            min_age_minutes: 10,
        },
        CleanupRule {
            id: "windows-temp",
            title: "Windows 临时文件",
            description: "系统和安装程序留下的临时材料，需要管理员权限；V0.1 只展示能力说明，不执行提权清理。",
            source_category: SourceCategory::System,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::RequiresAdmin,
            default_selected: false,
            scope: RuleScope::WindowsRelative("Temp"),
            delete_contents_only: true,
            min_age_minutes: 30,
        },
        CleanupRule {
            id: "windows-update-download",
            title: "Windows 更新下载缓存",
            description: "Windows 更新下载后的缓存文件，需要管理员权限；V0.1 只展示能力说明，不执行提权清理。",
            source_category: SourceCategory::System,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::RequiresAdmin,
            default_selected: false,
            scope: RuleScope::WindowsRelative("SoftwareDistribution\\Download"),
            delete_contents_only: true,
            min_age_minutes: 60,
        },
        CleanupRule {
            id: "wechat-data-root",
            title: "微信数据根目录",
            description: "微信数据根目录可能包含聊天数据库、图片、视频和文件。V0.1 不提供整目录删除。",
            source_category: SourceCategory::Wechat,
            risk_level: RiskLevel::NotCleanable,
            cleanup_action: CleanupAction::ExplainOnly,
            default_selected: false,
            scope: RuleScope::UserProfileRelative("Documents\\WeChat Files"),
            delete_contents_only: false,
            min_age_minutes: 0,
        },
        CleanupRule {
            id: "qq-data-root",
            title: "QQ 数据根目录",
            description: "QQ 数据根目录可能包含聊天数据库、图片、视频、群文件和下载文件。V0.1 不提供整目录删除。",
            source_category: SourceCategory::Qq,
            risk_level: RiskLevel::NotCleanable,
            cleanup_action: CleanupAction::ExplainOnly,
            default_selected: false,
            scope: RuleScope::UserProfileRelative("Documents\\Tencent Files"),
            delete_contents_only: false,
            min_age_minutes: 0,
        },
        CleanupRule {
            id: "vscode-cached-vsix",
            title: "VS Code 扩展安装包缓存",
            description: "VS Code 下载扩展时留下的安装包缓存，可重新下载。",
            source_category: SourceCategory::InstallersOldVersions,
            risk_level: RiskLevel::Recommended,
            cleanup_action: CleanupAction::DirectDelete,
            default_selected: true,
            scope: RuleScope::UserProfileRelative("AppData\\Roaming\\Code\\CachedExtensionVSIXs"),
            delete_contents_only: false,
            min_age_minutes: 10,
        },
    ]
}
```

Create `src-tauri/src/fixtures.rs`:

```rust
use time::OffsetDateTime;

pub fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
```

- [ ] **Step 4: Export modules**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod fixtures;
pub mod models;
pub mod rules;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 5: Run rule registry tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml rule_registry -- --nocapture
```

Expected: all rule registry tests pass.

- [ ] **Step 6: Commit rules**

Run:

```powershell
git add src-tauri/src/rules.rs src-tauri/src/fixtures.rs src-tauri/tests/rule_registry_tests.rs src-tauri/src/lib.rs
git commit -m "feat: add bundled cleanup rules"
```

Expected: commit succeeds.

## Task 5: Implement Path Resolution And Size Calculation

**Files:**

- Create: `src-tauri/src/errors.rs`
- Create: `src-tauri/src/drive.rs`
- Create: `src-tauri/src/paths.rs`
- Create: `src-tauri/src/size.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add tempdir test dependency**

Modify `src-tauri/Cargo.toml` and add:

```toml
[dev-dependencies]
tempfile = "3.14"
```

- [ ] **Step 2: Create error type**

Create `src-tauri/src/errors.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CleanerError {
    #[error("path is outside the allowed root")]
    PathOutsideAllowedRoot,
    #[error("path cannot be resolved: {0}")]
    PathResolution(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for CleanerError {
    fn from(value: std::io::Error) -> Self {
        CleanerError::Io(value.to_string())
    }
}
```

- [ ] **Step 3: Create path resolver**

Create `src-tauri/src/drive.rs`:

```rust
use sysinfo::Disks;

use crate::errors::CleanerError;
use crate::models::DriveSummary;

pub fn c_drive_summary() -> Result<DriveSummary, CleanerError> {
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy().replace('/', "\\").to_uppercase();
        if mount == "C:\\" || mount == "C:" {
            return Ok(DriveSummary {
                drive: "C:".to_string(),
                total_bytes: disk.total_space(),
                free_bytes: disk.available_space(),
            });
        }
    }
    Err(CleanerError::PathResolution("C drive was not found".to_string()))
}
```

- [ ] **Step 4: Create path resolver**

Create `src-tauri/src/paths.rs`:

```rust
use std::path::{Path, PathBuf};

use crate::errors::CleanerError;
use crate::rules::{CleanupRule, RuleScope};

#[derive(Debug, Clone)]
pub struct ScanRoots {
    pub c_drive: PathBuf,
    pub user_profile: PathBuf,
    pub local_app_data: PathBuf,
    pub windows_dir: PathBuf,
}

impl ScanRoots {
    pub fn from_current_user() -> Result<Self, CleanerError> {
        let user_profile = std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .ok_or_else(|| CleanerError::PathResolution("USERPROFILE is not set".to_string()))?;
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| CleanerError::PathResolution("LOCALAPPDATA is not set".to_string()))?;
        Ok(Self {
            c_drive: PathBuf::from(r"C:\"),
            user_profile,
            local_app_data,
            windows_dir: PathBuf::from(r"C:\Windows"),
        })
    }
}

pub fn resolve_rule_path(rule: &CleanupRule, roots: &ScanRoots) -> PathBuf {
    match &rule.scope {
        RuleScope::UserLocalAppDataRelative(relative) => roots.local_app_data.join(relative),
        RuleScope::UserProfileRelative(relative) => roots.user_profile.join(relative),
        RuleScope::WindowsRelative(relative) => roots.windows_dir.join(relative),
        RuleScope::Absolute(path) => PathBuf::from(path),
    }
}

pub fn root_for_rule(rule: &CleanupRule, roots: &ScanRoots) -> PathBuf {
    match &rule.scope {
        RuleScope::UserLocalAppDataRelative(_) => roots.local_app_data.clone(),
        RuleScope::UserProfileRelative(_) => roots.user_profile.clone(),
        RuleScope::WindowsRelative(_) => roots.windows_dir.clone(),
        RuleScope::Absolute(_) => roots.c_drive.clone(),
    }
}

pub fn ensure_under_root(path: &Path, root: &Path) -> Result<(), CleanerError> {
    let path = path.canonicalize()?;
    let root = root.canonicalize()?;
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(CleanerError::PathOutsideAllowedRoot)
    }
}
```

- [ ] **Step 5: Create size calculator**

Create `src-tauri/src/size.rs`:

```rust
use std::path::Path;
use walkdir::WalkDir;

pub fn path_size_bytes(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }

    if path.is_file() {
        return path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    }

    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum()
}
```

- [ ] **Step 6: Export modules**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod rules;
pub mod size;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 7: Verify compile**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust tests pass.

- [ ] **Step 8: Commit path utilities**

Run:

```powershell
git add src-tauri/Cargo.toml src-tauri/src/errors.rs src-tauri/src/drive.rs src-tauri/src/paths.rs src-tauri/src/size.rs src-tauri/src/lib.rs
git commit -m "feat: add path and size utilities"
```

Expected: commit succeeds.

## Task 6: Implement Configuration Reference Protection

**Files:**

- Create: `src-tauri/src/config_refs.rs`
- Create: `src-tauri/tests/config_refs_tests.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write config reference tests**

Create `src-tauri/tests/config_refs_tests.rs`:

```rust
use std::fs;

use c_drive_cleaner::config_refs::{find_config_references, ConfigSearchRoots};

#[test]
fn detects_candidate_path_inside_plain_text_config() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let codex_dir = user_profile.join(".codex");
    let candidate = user_profile.join(".vscode").join("extensions").join("highagency.pencildev-0.6.51");

    fs::create_dir_all(&codex_dir).expect("codex dir");
    fs::create_dir_all(&candidate).expect("candidate dir");
    fs::write(
        codex_dir.join("config.toml"),
        format!("command = '{}\\\\out\\\\mcp-server-windows-x64.exe'", candidate.display()),
    )
    .expect("config");

    let refs = find_config_references(
        &candidate,
        &ConfigSearchRoots {
            user_profile: user_profile.clone(),
        },
    );

    assert_eq!(refs.len(), 1);
    assert!(refs[0].display_name.contains(".codex"));
}

#[test]
fn detects_candidate_path_inside_root_claude_json_with_escaped_backslashes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let candidate = user_profile.join("AppData").join("Local").join("npm-cache").join("_npx").join("abc");
    let escaped = candidate.to_string_lossy().replace('\\', "\\\\");

    fs::create_dir_all(&candidate).expect("candidate dir");
    fs::create_dir_all(&user_profile).expect("user profile");
    fs::write(
        user_profile.join(".claude.json"),
        format!(r#"{{"args":["{}\\node_modules\\exa-mcp-server\\index.cjs"]}}"#, escaped),
    )
    .expect("claude json");

    let refs = find_config_references(
        &candidate,
        &ConfigSearchRoots {
            user_profile: user_profile.clone(),
        },
    );

    assert_eq!(refs.len(), 1);
    assert!(refs[0].display_name.contains(".claude.json"));
}

#[test]
fn ignores_binary_like_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let user_profile = temp.path().join("user");
    let codex_dir = user_profile.join(".codex");
    let candidate = user_profile.join("AppData").join("Local").join("npm-cache");

    fs::create_dir_all(&codex_dir).expect("codex dir");
    fs::create_dir_all(&candidate).expect("candidate dir");
    fs::write(codex_dir.join("blob.bin"), [0_u8, 159, 146, 150]).expect("binary");

    let refs = find_config_references(
        &candidate,
        &ConfigSearchRoots {
            user_profile: user_profile.clone(),
        },
    );

    assert!(refs.is_empty());
}
```

- [ ] **Step 2: Run config reference tests and verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml config_refs -- --nocapture
```

Expected: tests fail because `config_refs` does not exist.

- [ ] **Step 3: Implement config reference scanner**

Create `src-tauri/src/config_refs.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSearchRoots {
    pub user_profile: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigReference {
    pub display_name: String,
}

pub fn find_config_references(candidate_path: &Path, roots: &ConfigSearchRoots) -> Vec<ConfigReference> {
    let candidate = normalize_for_search(candidate_path);
    let known_dirs = [
        roots.user_profile.join(".codex"),
        roots.user_profile.join(".claude"),
        roots.user_profile.join(".cursor"),
        roots.user_profile.join(".vscode"),
        roots.user_profile.join(".trae"),
    ];
    let known_files = [
        roots.user_profile.join(".claude.json"),
        roots.user_profile.join(".codex.json"),
    ];

    let mut refs = Vec::new();
    for file in known_files {
        scan_config_file(&file, &candidate, roots, &mut refs);
    }

    for dir in known_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(&dir)
            .max_depth(4)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            scan_config_file(entry.path(), &candidate, roots, &mut refs);
        }
    }

    refs
}

fn scan_config_file(path: &Path, candidate: &str, roots: &ConfigSearchRoots, refs: &mut Vec<ConfigReference>) {
    if !path.exists() || !is_plain_text_candidate(path) {
        return;
    }

    if let Ok(bytes) = fs::read(path) {
        if looks_binary(&bytes) {
            return;
        }
        let text = String::from_utf8_lossy(&bytes);
        if normalize_for_search_text(&text).contains(candidate) {
            refs.push(ConfigReference {
                display_name: path_display_without_username(path, &roots.user_profile),
            });
        }
    }
}

fn is_plain_text_candidate(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|value| value.to_str()).unwrap_or(""),
        "json" | "jsonl" | "toml" | "yaml" | "yml" | "ini" | "txt" | "config" | "conf"
    )
}

fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(512).any(|byte| *byte == 0)
}

fn normalize_for_search(path: &Path) -> String {
    normalize_slashes(path.to_string_lossy().replace('/', "\\").to_lowercase())
}

fn normalize_for_search_text(text: &str) -> String {
    normalize_slashes(text.replace('/', "\\").to_lowercase())
}

fn normalize_slashes(input: String) -> String {
    let mut output = String::with_capacity(input.len());
    let mut previous_was_slash = false;
    for character in input.chars() {
        if character == '\\' {
            if !previous_was_slash {
                output.push(character);
            }
            previous_was_slash = true;
        } else {
            output.push(character);
            previous_was_slash = false;
        }
    }
    output
}

fn path_display_without_username(path: &Path, user_profile: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(user_profile) {
        format!("用户配置\\{}", relative.display())
    } else {
        "用户配置文件".to_string()
    }
}
```

- [ ] **Step 4: Export module**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod rules;
pub mod size;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 5: Run config reference tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml config_refs -- --nocapture
```

Expected: both config reference tests pass.

- [ ] **Step 6: Commit config reference protection**

Run:

```powershell
git add src-tauri/src/config_refs.rs src-tauri/tests/config_refs_tests.rs src-tauri/src/lib.rs
git commit -m "feat: detect config references before cleanup"
```

Expected: commit succeeds.

## Task 7: Implement Process Usage Protection

**Files:**

- Create: `src-tauri/src/processes.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement process usage scanner**

Create `src-tauri/src/processes.rs`:

```rust
use std::path::Path;

use sysinfo::{ProcessesToUpdate, System};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessReference {
    pub process_name: String,
}

pub fn find_process_references(candidate_path: &Path) -> Vec<ProcessReference> {
    let candidate = normalize(candidate_path);
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut refs = Vec::new();
    for process in system.processes().values() {
        let exe_matches = process
            .exe()
            .map(|path| normalize(path).starts_with(&candidate))
            .unwrap_or(false);
        let cmd_matches = process.cmd().iter().any(|part| part.to_string_lossy().to_lowercase().contains(&candidate));

        if exe_matches || cmd_matches {
            refs.push(ProcessReference {
                process_name: process.name().to_string_lossy().to_string(),
            });
        }
    }

    refs.sort_by(|left, right| left.process_name.cmp(&right.process_name));
    refs.dedup();
    refs
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\").to_lowercase()
}
```

- [ ] **Step 2: Export module**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod processes;
pub mod rules;
pub mod size;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 3: Verify compile**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust tests pass.

- [ ] **Step 4: Commit process protection**

Run:

```powershell
git add src-tauri/src/processes.rs src-tauri/src/lib.rs
git commit -m "feat: detect running processes using cleanup paths"
```

Expected: commit succeeds.

## Task 8: Implement Scan Report Generation

**Files:**

- Create: `src-tauri/src/scan.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement scan engine**

Create `src-tauri/src/scan.rs`:

```rust
use crate::config_refs::{find_config_references, ConfigSearchRoots};
use crate::fixtures::now_iso;
use crate::models::{CleanupAction, DriveSummary, RiskLevel, ScanItem, ScanReport};
use crate::paths::{resolve_rule_path, ScanRoots};
use crate::processes::find_process_references;
use crate::rules::{builtin_rules, CleanupRule};
use crate::size::path_size_bytes;

pub fn scan_with_roots(roots: &ScanRoots, drive_summary: DriveSummary) -> ScanReport {
    let started = now_iso();
    let mut items = Vec::new();

    for rule in builtin_rules() {
        let path = resolve_rule_path(&rule, roots);
        let estimated_bytes = path_size_bytes(&path);
        if estimated_bytes == 0 && !path.exists() {
            continue;
        }
        items.push(build_item(rule, roots, path.to_string_lossy().to_string(), estimated_bytes));
    }

    ScanReport {
        drive_summary,
        items,
        partial: false,
        scan_started_at: started,
        scan_finished_at: now_iso(),
    }
}

fn build_item(rule: CleanupRule, roots: &ScanRoots, technical_path: String, estimated_bytes: u64) -> ScanItem {
    let config_refs = find_config_references(
        std::path::Path::new(&technical_path),
        &ConfigSearchRoots {
            user_profile: roots.user_profile.clone(),
        },
    );
    let process_refs = find_process_references(std::path::Path::new(&technical_path));

    let mut risk_level = rule.risk_level.clone();
    let mut cleanup_action = rule.cleanup_action.clone();
    let mut default_selected = rule.default_selected;
    let mut reasons = vec![format!("规则命中：{}", rule.title)];
    let mut warnings = Vec::new();

    if !config_refs.is_empty() {
        risk_level = RiskLevel::NotCleanable;
        cleanup_action = CleanupAction::BlockedByConfigReference;
        default_selected = false;
        reasons.push("这个位置正在被工具配置引用。".to_string());
        warnings.push("清理可能导致工具启动失败，已自动跳过。".to_string());
    } else if !process_refs.is_empty() {
        risk_level = RiskLevel::NotCleanable;
        cleanup_action = CleanupAction::BlockedByProcess;
        default_selected = false;
        reasons.push("这个位置正在被运行中的程序使用。".to_string());
        warnings.push("请关闭相关软件后重新扫描。".to_string());
    }

    ScanItem {
        id: rule.id.to_string(),
        title: rule.title.to_string(),
        description: rule.description.to_string(),
        source_category: rule.source_category,
        risk_level,
        cleanup_action,
        estimated_bytes,
        default_selected,
        user_visible_path_hint: user_visible_hint(&rule.id),
        technical_path: Some(technical_path),
        reasons,
        warnings,
    }
}

fn user_visible_hint(rule_id: &str) -> String {
    match rule_id {
        "user-temp" => "当前用户临时目录".to_string(),
        "windows-temp" => "Windows 临时目录".to_string(),
        "windows-update-download" => "Windows 更新下载缓存".to_string(),
        "wechat-data-root" => "微信用户数据根目录".to_string(),
        "qq-data-root" => "QQ 用户数据根目录".to_string(),
        "vscode-cached-vsix" => "VS Code 扩展安装包缓存".to_string(),
        _ => "C 盘应用数据目录".to_string(),
    }
}
```

- [ ] **Step 2: Add Tauri scan command**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod processes;
pub mod rules;
pub mod scan;
pub mod size;

use models::ScanReport;
use paths::ScanRoots;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

#[tauri::command]
fn scan_c_drive() -> Result<ScanReport, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    Ok(scan::scan_with_roots(&roots, drive_summary))
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping, scan_c_drive])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 3: Verify scan command compiles**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Expected: both commands exit 0.

- [ ] **Step 4: Commit scan generation**

Run:

```powershell
git add src-tauri/src/scan.rs src-tauri/src/lib.rs
git commit -m "feat: generate cleanup scan reports"
```

Expected: commit succeeds.

## Task 9: Implement Cleanup Executor With Revalidation

**Files:**

- Create: `src-tauri/src/cleanup.rs`
- Create: `src-tauri/tests/cleanup_tests.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write cleanup tests**

Create `src-tauri/tests/cleanup_tests.rs`:

```rust
use std::fs;

use c_drive_cleaner::cleanup::{delete_path_contents, validate_high_risk_confirmation};
use c_drive_cleaner::models::{CleanupSelection, RiskLevel, ScanItem, SourceCategory, CleanupAction};

#[test]
fn deletes_contents_without_deleting_parent_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let parent = temp.path().join("Temp");
    fs::create_dir_all(&parent).expect("parent");
    fs::write(parent.join("old.log"), "abc").expect("file");

    let freed = delete_path_contents(&parent).expect("deleted");

    assert!(parent.exists());
    assert!(!parent.join("old.log").exists());
    assert_eq!(freed, 3);
}

#[test]
fn rejects_high_risk_without_second_confirmation() {
    let item = ScanItem {
        id: "wechat-video-cache".to_string(),
        title: "微信视频缓存子目录".to_string(),
        description: "精确命中的微信视频缓存。".to_string(),
        source_category: SourceCategory::Wechat,
        risk_level: RiskLevel::HighRisk,
        cleanup_action: CleanupAction::DirectDelete,
        estimated_bytes: 10,
        default_selected: false,
        user_visible_path_hint: "微信视频缓存子目录".to_string(),
        technical_path: None,
        reasons: vec![],
        warnings: vec![],
    };
    let selection = CleanupSelection {
        selected_item_ids: vec!["wechat-video-cache".to_string()],
        high_risk_confirmed: false,
        request_admin_mode: false,
    };

    assert!(validate_high_risk_confirmation(&selection, &[item]).is_err());
}
```

- [ ] **Step 2: Run cleanup tests and verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml cleanup -- --nocapture
```

Expected: tests fail because `cleanup` does not exist.

- [ ] **Step 3: Implement cleanup executor helpers**

Create `src-tauri/src/cleanup.rs`:

```rust
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::config_refs::{find_config_references, ConfigSearchRoots};
use crate::errors::CleanerError;
use crate::models::{CleanupAction, CleanupItemResult, CleanupResult, CleanupSelection, RiskLevel, ScanItem};
use crate::fixtures::now_iso;
use crate::paths::{ensure_under_root, resolve_rule_path, root_for_rule, ScanRoots};
use crate::processes::find_process_references;
use crate::rules::builtin_rules;

pub fn validate_high_risk_confirmation(selection: &CleanupSelection, items: &[ScanItem]) -> Result<(), String> {
    let selected: std::collections::HashSet<&str> = selection.selected_item_ids.iter().map(String::as_str).collect();
    let has_high_risk = items
        .iter()
        .any(|item| selected.contains(item.id.as_str()) && item.risk_level == RiskLevel::HighRisk);

    if has_high_risk && !selection.high_risk_confirmed {
        Err("高风险项目需要二次确认。".to_string())
    } else {
        Ok(())
    }
}

pub fn delete_path_contents(path: &Path) -> Result<u64, CleanerError> {
    let mut freed = 0;
    if !path.exists() {
        return Ok(0);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        let size = crate::size::path_size_bytes(&child);
        if child.is_dir() {
            fs::remove_dir_all(&child)?;
        } else {
            fs::remove_file(&child)?;
        }
        freed += size;
    }

    Ok(freed)
}

pub fn delete_path_or_contents(path: &Path, contents_only: bool) -> Result<u64, CleanerError> {
    if contents_only {
        return delete_path_contents(path);
    }

    let freed = crate::size::path_size_bytes(path);
    if !path.exists() {
        return Ok(0);
    }
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(freed)
}

pub fn execute_selected_cleanup(selection: &CleanupSelection, items: &[ScanItem], roots: &ScanRoots) -> Result<CleanupResult, String> {
    validate_high_risk_confirmation(selection, items)?;
    let selected: std::collections::HashSet<&str> = selection.selected_item_ids.iter().map(String::as_str).collect();
    let rules = builtin_rules();
    let mut results = Vec::new();

    for item in items.iter().filter(|item| selected.contains(item.id.as_str())) {
        let Some(rule) = rules.iter().find(|rule| rule.id == item.id) else {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：找不到对应安全规则。", item.title),
            });
            continue;
        };

        if item.risk_level == RiskLevel::NotCleanable {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：安全规则不允许清理。", item.title),
            });
            continue;
        }

        if item.cleanup_action != CleanupAction::DirectDelete || rule.cleanup_action != CleanupAction::DirectDelete {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：需要其他处理方式。", item.title),
            });
            continue;
        }

        let Some(path) = item.technical_path.as_ref() else {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：缺少路径信息。", item.title),
            });
            continue;
        };

        let expected_path = resolve_rule_path(rule, roots);
        if Path::new(path) != expected_path {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：扫描路径和规则路径不一致。", item.title),
            });
            continue;
        }

        if !expected_path.exists() {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：路径已不存在。", item.title),
            });
            continue;
        }

        if let Err(error) = ensure_under_root(&expected_path, &root_for_rule(rule, roots)) {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：路径安全校验未通过：{}。", item.title, error),
            });
            continue;
        }

        if !find_config_references(
            &expected_path,
            &ConfigSearchRoots {
                user_profile: roots.user_profile.clone(),
            },
        )
        .is_empty()
        {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：清理前复查发现仍被配置引用。", item.title),
            });
            continue;
        }

        if !find_process_references(&expected_path).is_empty() {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：清理前复查发现仍被运行中的程序使用。", item.title),
            });
            continue;
        }

        if !path_is_old_enough(&expected_path, rule.min_age_minutes) {
            results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "skipped".to_string(),
                freed_bytes: 0,
                message: format!("{} 已跳过：最近仍有修改，暂不清理。", item.title),
            });
            continue;
        }

        match delete_path_or_contents(&expected_path, rule.delete_contents_only) {
            Ok(freed) => results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "deleted".to_string(),
                freed_bytes: freed,
                message: format!("{} 已清理。", item.title),
            }),
            Err(error) => results.push(CleanupItemResult {
                item_id: item.id.clone(),
                status: "failed".to_string(),
                freed_bytes: 0,
                message: format!("{} 清理失败：{}。", item.title, error),
            }),
        }
    }

    Ok(build_cleanup_result(results))
}

fn path_is_old_enough(path: &Path, min_age_minutes: u64) -> bool {
    if min_age_minutes == 0 {
        return true;
    }

    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(min_age_minutes * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    newest_modified_time(path)
        .map(|modified| modified <= cutoff)
        .unwrap_or(false)
}

fn newest_modified_time(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return path.metadata().ok().and_then(|metadata| metadata.modified().ok());
    }

    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter_map(|metadata| metadata.modified().ok())
        .max()
}

pub fn build_cleanup_result(results: Vec<CleanupItemResult>) -> CleanupResult {
    let total = results.iter().map(|result| result.freed_bytes).sum();
    CleanupResult {
        results,
        total_freed_bytes: total,
        finished_at: now_iso(),
    }
}
```

- [ ] **Step 4: Add Tauri cleanup command shell**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod cleanup;
pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod processes;
pub mod rules;
pub mod scan;
pub mod size;

use models::{CleanupResult, CleanupSelection, ScanReport};
use paths::ScanRoots;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

#[tauri::command]
fn scan_c_drive() -> Result<ScanReport, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    Ok(scan::scan_with_roots(&roots, drive_summary))
}

#[tauri::command]
fn execute_cleanup(selection: CleanupSelection) -> Result<CleanupResult, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    let report = scan::scan_with_roots(&roots, drive_summary);
    cleanup::execute_selected_cleanup(&selection, &report.items, &roots)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping, scan_c_drive, execute_cleanup])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 5: Run cleanup tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml cleanup -- --nocapture
```

Expected: cleanup tests pass.

- [ ] **Step 6: Commit cleanup foundation**

Run:

```powershell
git add src-tauri/src/cleanup.rs src-tauri/tests/cleanup_tests.rs src-tauri/src/lib.rs
git commit -m "feat: add cleanup executor safeguards"
```

Expected: commit succeeds.

## Task 10: Implement Tauri API Bridge And Mock Fallback

**Files:**

- Create: `src/services/tauriApi.ts`
- Create: `src/services/mockReport.ts`

- [ ] **Step 1: Create mock scan report**

Create `src/services/mockReport.ts`:

```ts
import type { ScanReport } from "../domain/models";

export const mockReport: ScanReport = {
  driveSummary: {
    drive: "C:",
    totalBytes: 251535081472,
    freeBytes: 27938717696,
  },
  partial: false,
  scanStartedAt: "2026-06-11T00:00:00Z",
  scanFinishedAt: "2026-06-11T00:00:05Z",
  items: [
    {
      id: "user-temp",
      title: "用户临时文件",
      description: "软件运行时留下的临时材料，通常可以安全删除。",
      sourceCategory: "system",
      riskLevel: "recommended",
      cleanupAction: "directDelete",
      estimatedBytes: 386924544,
      defaultSelected: true,
      userVisiblePathHint: "当前用户临时目录",
      reasons: ["命中安全白名单。"],
      warnings: [],
    },
    {
      id: "wechat-video-cache",
      title: "微信视频缓存子目录",
      description: "精确命中的微信视频缓存子目录，可能包含你仍想保留的视频。",
      sourceCategory: "wechat",
      riskLevel: "highRisk",
      cleanupAction: "directDelete",
      estimatedBytes: 980000000,
      defaultSelected: false,
      userVisiblePathHint: "微信视频缓存子目录",
      reasons: ["命中精确子目录规则，但仍属于用户数据。"],
      warnings: ["可能删除聊天中的视频缓存，删除后无法保证恢复。"],
    },
    {
      id: "wechat-data-root",
      title: "微信数据根目录",
      description: "微信完整数据目录包含聊天记录、图片、视频、文件和数据库。",
      sourceCategory: "wechat",
      riskLevel: "notCleanable",
      cleanupAction: "explainOnly",
      estimatedBytes: 4620000000,
      defaultSelected: false,
      userVisiblePathHint: "微信用户数据根目录",
      reasons: ["V0.1 不提供聊天软件根目录删除。"],
      warnings: ["请在微信内置设置中管理聊天文件，或等待后续版本提供更细粒度分类。"],
    },
    {
      id: "tool-config-ref",
      title: "工具旧版本运行目录",
      description: "这个目录看起来像旧版本，但某个工具仍从这里启动。",
      sourceCategory: "installersOldVersions",
      riskLevel: "notCleanable",
      cleanupAction: "blockedByConfigReference",
      estimatedBytes: 640000000,
      defaultSelected: false,
      userVisiblePathHint: "工具扩展目录",
      reasons: ["被工具配置引用。"],
      warnings: ["清理可能导致工具启动失败，已自动跳过。"],
    },
  ],
};
```

- [ ] **Step 2: Create Tauri API wrapper**

Create `src/services/tauriApi.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { CleanupResult, CleanupSelection, ScanReport } from "../domain/models";
import { mockReport } from "./mockReport";

const isBrowserPreview = typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

export async function scanCDrive(): Promise<ScanReport> {
  if (isBrowserPreview) {
    return mockReport;
  }
  return invoke<ScanReport>("scan_c_drive");
}

export async function executeCleanup(selection: CleanupSelection): Promise<CleanupResult> {
  if (isBrowserPreview) {
    return {
      results: selection.selectedItemIds.map((id) => ({
        itemId: id,
        status: "deleted",
        freedBytes: mockReport.items.find((item) => item.id === id)?.estimatedBytes ?? 0,
        message: "浏览器预览模式模拟清理成功。",
      })),
      totalFreedBytes: selection.selectedItemIds.reduce(
        (total, id) => total + (mockReport.items.find((item) => item.id === id)?.estimatedBytes ?? 0),
        0,
      ),
      finishedAt: new Date().toISOString(),
    };
  }
  return invoke<CleanupResult>("execute_cleanup", { selection });
}
```

- [ ] **Step 3: Verify TypeScript build**

Run:

```powershell
npm run build
```

Expected: build exits 0.

- [ ] **Step 4: Commit API bridge**

Run:

```powershell
git add src/services
git commit -m "feat: add frontend tauri api bridge"
```

Expected: commit succeeds.

## Task 11: Build Wizard UI

**Files:**

- Create: `src/components/AppShell.tsx`
- Create: `src/components/StepIndicator.tsx`
- Create: `src/components/WelcomeStep.tsx`
- Create: `src/components/ScanStep.tsx`
- Create: `src/components/SuggestionsStep.tsx`
- Create: `src/components/ConfirmStep.tsx`
- Create: `src/components/CleanStep.tsx`
- Create: `src/components/ResultStep.tsx`
- Create: `src/components/ErrorPanel.tsx`
- Create: `src/components/RiskBadge.tsx`
- Create: `src/components/CleanupItemRow.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/theme.css`

- [ ] **Step 1: Create shared shell and step indicator**

Create `src/components/StepIndicator.tsx`:

```tsx
const steps = ["扫描", "建议", "确认", "清理", "结果"] as const;

export function StepIndicator({ current }: { current: number }) {
  return (
    <nav className="step-indicator" aria-label="清理流程">
      {steps.map((step, index) => (
        <div className="step-item" data-active={index === current} data-done={index < current} key={step}>
          <span>{index + 1}</span>
          <strong>{step}</strong>
        </div>
      ))}
    </nav>
  );
}
```

Create `src/components/AppShell.tsx`:

```tsx
import type { ReactNode } from "react";
import { StepIndicator } from "./StepIndicator";

export function AppShell({ currentStep, children }: { currentStep: number; children: ReactNode }) {
  return (
    <main className="app-frame">
      <aside className="app-sidebar">
        <p className="eyebrow">C Drive Cleaner</p>
        <h1>安全清理 C 盘</h1>
        <p>先分析，再解释，最后由你确认清理。</p>
        <StepIndicator current={currentStep} />
      </aside>
      <section className="app-panel">{children}</section>
    </main>
  );
}
```

- [ ] **Step 2: Create risk badge and item row**

Create `src/components/RiskBadge.tsx`:

```tsx
import type { RiskLevel } from "../domain/models";

const labels: Record<RiskLevel, string> = {
  recommended: "推荐",
  optional: "可选",
  highRisk: "高风险",
  notCleanable: "不可清理",
};

export function RiskBadge({ risk }: { risk: RiskLevel }) {
  return (
    <span className="risk-badge" data-risk={risk}>
      {labels[risk]}
    </span>
  );
}
```

Create `src/components/CleanupItemRow.tsx`:

```tsx
import type { ScanItem } from "../domain/models";
import { RiskBadge } from "./RiskBadge";

export function CleanupItemRow({
  item,
  checked,
  onCheckedChange,
}: {
  item: ScanItem;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  const selectable = item.riskLevel !== "notCleanable";
  return (
    <article className="cleanup-row" data-disabled={!selectable}>
      <label className="cleanup-check">
        <input
          type="checkbox"
          disabled={!selectable}
          checked={selectable && checked}
          onChange={(event) => onCheckedChange(event.target.checked)}
        />
      </label>
      <div className="cleanup-main">
        <div className="cleanup-title-line">
          <h3>{item.title}</h3>
          <RiskBadge risk={item.riskLevel} />
        </div>
        <p>{item.description}</p>
        <p className="path-hint">{item.userVisiblePathHint}</p>
        {item.reasons.map((reason) => (
          <p className="reason" key={reason}>
            {reason}
          </p>
        ))}
        {item.warnings.map((warning) => (
          <p className="warning" key={warning}>
            {warning}
          </p>
        ))}
      </div>
      <strong className="bytes">{formatBytes(item.estimatedBytes)}</strong>
    </article>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) {
    return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  }
  if (bytes >= 1024 ** 2) {
    return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  }
  return `${bytes} B`;
}
```

- [ ] **Step 3: Create wizard step components**

Create `src/components/WelcomeStep.tsx`:

```tsx
export function WelcomeStep({ onStart }: { onStart: () => void }) {
  return (
    <div className="step-content">
      <p className="eyebrow">开始前</p>
      <h2>先扫描，不会直接删除任何文件</h2>
      <p>软件会检查 C 盘里的临时文件、缓存、安装包、常见软件数据和配置引用风险。</p>
      <div className="assurance-grid">
        <div>默认只勾选推荐清理项</div>
        <div>高风险项目需要二次确认</div>
        <div>被配置引用的目录不会清理</div>
      </div>
      <button className="primary-button" onClick={onStart}>
        开始扫描 C 盘
      </button>
    </div>
  );
}
```

Create `src/components/ScanStep.tsx`:

```tsx
export function ScanStep() {
  return (
    <div className="step-content">
      <p className="eyebrow">扫描中</p>
      <h2>正在分析 C 盘可清理项目</h2>
      <div className="progress-track">
        <div className="progress-bar" />
      </div>
      <ul className="scan-list">
        <li>临时文件和下载缓存</li>
        <li>安装包和旧版本</li>
        <li>常见软件缓存</li>
        <li>配置引用和进程占用</li>
      </ul>
    </div>
  );
}
```

Create `src/components/SuggestionsStep.tsx`:

```tsx
import type { ScanItem } from "../domain/models";
import { groupByRisk, groupBySource } from "../domain/grouping";
import { toggleSelection } from "../domain/selection";
import { CleanupItemRow } from "./CleanupItemRow";

export function SuggestionsStep({
  items,
  selectedIds,
  view,
  onViewChange,
  onSelectionChange,
  onNext,
}: {
  items: ScanItem[];
  selectedIds: string[];
  view: "risk" | "source";
  onViewChange: (view: "risk" | "source") => void;
  onSelectionChange: (ids: string[]) => void;
  onNext: () => void;
}) {
  const grouped = view === "risk" ? groupByRisk(items) : groupBySource(items);
  return (
    <div className="step-content">
      <div className="split-title">
        <div>
          <p className="eyebrow">清理建议</p>
          <h2>按风险或来源查看同一批扫描结果</h2>
        </div>
        <div className="segmented">
          <button data-active={view === "risk"} onClick={() => onViewChange("risk")}>按风险</button>
          <button data-active={view === "source"} onClick={() => onViewChange("source")}>按来源</button>
        </div>
      </div>
      <div className="cleanup-list">
        {Object.entries(grouped).map(([group, groupItems]) => (
          <section className="cleanup-group" key={group}>
            <h3>{groupLabel(group)}</h3>
            {groupItems.map((item) => (
              <CleanupItemRow
                item={item}
                checked={selectedIds.includes(item.id)}
                key={item.id}
                onCheckedChange={(checked) => onSelectionChange(toggleSelection(selectedIds, item, checked))}
              />
            ))}
          </section>
        ))}
      </div>
      <button className="primary-button" onClick={onNext}>确认已选项目</button>
    </div>
  );
}

function groupLabel(group: string): string {
  const labels: Record<string, string> = {
    recommended: "推荐清理",
    optional: "可选清理",
    highRisk: "高风险清理",
    notCleanable: "不可清理",
    system: "系统清理",
    commonSoftware: "常用软件",
    wechat: "微信",
    qq: "QQ",
    workChat: "飞书 / 钉钉 / 企业微信",
    cloudDrive: "网盘与同步盘",
    installersOldVersions: "安装包与旧版本",
    otherLarge: "其他可疑大项",
  };
  return labels[group] ?? group;
}
```

Create `src/components/ConfirmStep.tsx`:

```tsx
import type { ScanItem } from "../domain/models";
import { estimateSelectedBytes, requiresHighRiskConfirmation } from "../domain/selection";

export function ConfirmStep({
  items,
  selectedIds,
  highRiskConfirmed,
  onHighRiskConfirmed,
  onBack,
  onConfirm,
}: {
  items: ScanItem[];
  selectedIds: string[];
  highRiskConfirmed: boolean;
  onHighRiskConfirmed: (confirmed: boolean) => void;
  onBack: () => void;
  onConfirm: () => void;
}) {
  const needsHighRisk = requiresHighRiskConfirmation(selectedIds, items);
  const selectedItems = items.filter((item) => selectedIds.includes(item.id));
  const canConfirm = selectedIds.length > 0 && (!needsHighRisk || highRiskConfirmed);

  return (
    <div className="step-content">
      <p className="eyebrow">确认清理</p>
      <h2>预计释放 {formatBytes(estimateSelectedBytes(selectedIds, items))}</h2>
      <div className="confirm-list">
        {selectedItems.map((item) => (
          <div className="confirm-row" key={item.id}>
            <span>{item.title}</span>
            <strong>{formatBytes(item.estimatedBytes)}</strong>
          </div>
        ))}
      </div>
      {needsHighRisk && (
        <label className="danger-confirm">
          <input checked={highRiskConfirmed} onChange={(event) => onHighRiskConfirmed(event.target.checked)} type="checkbox" />
          我理解高风险项目可能删除聊天文件、本地文件或离线数据，删除后无法保证恢复。
        </label>
      )}
      <div className="button-row">
        <button className="secondary-button" onClick={onBack}>返回修改</button>
        <button className="primary-button" disabled={!canConfirm} onClick={onConfirm}>确认清理已选项目</button>
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
```

Create `src/components/CleanStep.tsx`:

```tsx
export function CleanStep() {
  return (
    <div className="step-content">
      <p className="eyebrow">清理中</p>
      <h2>正在逐项清理并复查安全条件</h2>
      <div className="progress-track">
        <div className="progress-bar" />
      </div>
      <p>如果某一项失败，软件会继续处理其他项目，并在结果页说明原因。</p>
    </div>
  );
}
```

Create `src/components/ResultStep.tsx`:

```tsx
import type { CleanupResult } from "../domain/models";

export function ResultStep({ result, onRestart }: { result: CleanupResult; onRestart: () => void }) {
  return (
    <div className="step-content">
      <p className="eyebrow">完成</p>
      <h2>实际释放 {formatBytes(result.totalFreedBytes)}</h2>
      <div className="confirm-list">
        {result.results.map((item) => (
          <div className="confirm-row" key={item.itemId}>
            <span>{item.message}</span>
            <strong>{formatBytes(item.freedBytes)}</strong>
          </div>
        ))}
      </div>
      <button className="primary-button" onClick={onRestart}>重新扫描</button>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
```

Create `src/components/ErrorPanel.tsx`:

```tsx
export function ErrorPanel({
  message,
  onRetry,
  onDismiss,
}: {
  message: string;
  onRetry: () => void;
  onDismiss: () => void;
}) {
  return (
    <section className="error-panel" role="alert">
      <div>
        <p className="eyebrow">遇到问题</p>
        <h3>本次操作没有完成</h3>
        <p>{message}</p>
      </div>
      <div className="button-row">
        <button className="secondary-button" onClick={onDismiss}>先不处理</button>
        <button className="primary-button" onClick={onRetry}>重试</button>
      </div>
    </section>
  );
}
```

- [ ] **Step 4: Wire App state machine**

Modify `src/App.tsx`:

```tsx
import { useState } from "react";
import type { CleanupResult, ScanReport } from "./domain/models";
import { buildDefaultSelection } from "./domain/selection";
import { executeCleanup, scanCDrive } from "./services/tauriApi";
import { AppShell } from "./components/AppShell";
import { WelcomeStep } from "./components/WelcomeStep";
import { ScanStep } from "./components/ScanStep";
import { SuggestionsStep } from "./components/SuggestionsStep";
import { ConfirmStep } from "./components/ConfirmStep";
import { CleanStep } from "./components/CleanStep";
import { ResultStep } from "./components/ResultStep";
import { ErrorPanel } from "./components/ErrorPanel";

type Step = "welcome" | "scan" | "suggestions" | "confirm" | "clean" | "result";
type FailedAction = "scan" | "clean";

const stepIndex: Record<Step, number> = {
  welcome: 0,
  scan: 0,
  suggestions: 1,
  confirm: 2,
  clean: 3,
  result: 4,
};

export default function App() {
  const [step, setStep] = useState<Step>("welcome");
  const [report, setReport] = useState<ScanReport | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [view, setView] = useState<"risk" | "source">("risk");
  const [highRiskConfirmed, setHighRiskConfirmed] = useState(false);
  const [result, setResult] = useState<CleanupResult | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [failedAction, setFailedAction] = useState<FailedAction | null>(null);

  async function startScan() {
    setErrorMessage(null);
    setFailedAction(null);
    setStep("scan");
    try {
      const nextReport = await scanCDrive();
      setReport(nextReport);
      setSelectedIds(buildDefaultSelection(nextReport.items));
      setHighRiskConfirmed(false);
      setStep("suggestions");
    } catch (error) {
      setErrorMessage(toUserMessage(error));
      setFailedAction("scan");
      setStep("welcome");
    }
  }

  async function confirmCleanup() {
    if (!report) return;
    setErrorMessage(null);
    setFailedAction(null);
    setStep("clean");
    try {
      const nextResult = await executeCleanup({
        selectedItemIds: selectedIds,
        highRiskConfirmed,
        requestAdminMode: false,
      });
      setResult(nextResult);
      setStep("result");
    } catch (error) {
      setErrorMessage(toUserMessage(error));
      setFailedAction("clean");
      setStep("confirm");
    }
  }

  function restart() {
    setReport(null);
    setSelectedIds([]);
    setResult(null);
    setHighRiskConfirmed(false);
    setErrorMessage(null);
    setFailedAction(null);
    setStep("welcome");
  }

  function dismissError() {
    setErrorMessage(null);
    setFailedAction(null);
  }

  function retryFailedAction() {
    if (failedAction === "scan") {
      void startScan();
    }
    if (failedAction === "clean") {
      void confirmCleanup();
    }
  }

  return (
    <AppShell currentStep={stepIndex[step]}>
      {errorMessage && (
        <ErrorPanel message={errorMessage} onDismiss={dismissError} onRetry={retryFailedAction} />
      )}
      {step === "welcome" && <WelcomeStep onStart={startScan} />}
      {step === "scan" && <ScanStep />}
      {step === "suggestions" && report && (
        <SuggestionsStep
          items={report.items}
          selectedIds={selectedIds}
          view={view}
          onViewChange={setView}
          onSelectionChange={setSelectedIds}
          onNext={() => setStep("confirm")}
        />
      )}
      {step === "confirm" && report && (
        <ConfirmStep
          items={report.items}
          selectedIds={selectedIds}
          highRiskConfirmed={highRiskConfirmed}
          onHighRiskConfirmed={setHighRiskConfirmed}
          onBack={() => setStep("suggestions")}
          onConfirm={confirmCleanup}
        />
      )}
      {step === "clean" && <CleanStep />}
      {step === "result" && result && <ResultStep result={result} onRestart={restart} />}
    </AppShell>
  );
}

function toUserMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  if (typeof error === "string" && error.trim()) {
    return error;
  }
  return "系统返回了未知错误，本次没有删除任何新项目。";
}
```

- [ ] **Step 5: Replace CSS with polished app layout**

Modify `src/styles/theme.css` with a complete stylesheet:

```css
:root {
  color: #172026;
  background: #edf5f3;
  font-family: "Microsoft YaHei UI", "Segoe UI", sans-serif;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-width: 1024px;
  min-height: 720px;
}

button,
input {
  font: inherit;
}

button {
  border: 0;
}

.app-frame {
  min-height: 100vh;
  display: grid;
  grid-template-columns: 320px 1fr;
  background:
    linear-gradient(140deg, rgba(18, 122, 117, 0.14), transparent 38%),
    #edf5f3;
}

.app-sidebar {
  padding: 38px 30px;
  background: #0f2f35;
  color: #e8fffb;
}

.app-sidebar h1 {
  margin: 0 0 12px;
  font-size: 30px;
  line-height: 1.2;
}

.app-sidebar p {
  color: #a9c9c8;
  line-height: 1.7;
}

.app-panel {
  padding: 38px;
  overflow: auto;
}

.step-content {
  max-width: 920px;
  min-height: calc(100vh - 76px);
  background: rgba(255, 255, 255, 0.94);
  border: 1px solid #d4e3e0;
  border-radius: 8px;
  padding: 36px;
  box-shadow: 0 22px 70px rgba(20, 55, 58, 0.14);
}

.eyebrow {
  margin: 0 0 8px;
  color: #127a75;
  font-size: 13px;
  font-weight: 800;
  letter-spacing: 0;
  text-transform: uppercase;
}

h2 {
  margin: 0 0 14px;
  font-size: 30px;
  line-height: 1.25;
}

p {
  line-height: 1.7;
}

.step-indicator {
  display: grid;
  gap: 12px;
  margin-top: 34px;
}

.step-item {
  display: grid;
  grid-template-columns: 32px 1fr;
  align-items: center;
  gap: 10px;
  color: #8bb5b3;
}

.step-item span {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 1px solid #527c7b;
}

.step-item[data-active="true"],
.step-item[data-done="true"] {
  color: #ffffff;
}

.step-item[data-active="true"] span,
.step-item[data-done="true"] span {
  background: #22aaa2;
  border-color: #22aaa2;
}

.assurance-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin: 28px 0;
}

.assurance-grid div {
  border: 1px solid #d4e3e0;
  border-radius: 8px;
  padding: 16px;
  background: #f7fbfa;
  color: #31575a;
}

.primary-button,
.secondary-button {
  min-height: 44px;
  border-radius: 8px;
  padding: 0 18px;
  cursor: pointer;
}

.primary-button {
  background: #127a75;
  color: white;
}

.primary-button:disabled {
  cursor: not-allowed;
  background: #9bb9b6;
}

.secondary-button {
  background: #e5efed;
  color: #174247;
}

.progress-track {
  height: 12px;
  border-radius: 999px;
  background: #d9e8e5;
  overflow: hidden;
  margin: 28px 0;
}

.progress-bar {
  width: 62%;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #127a75, #36b9aa);
  animation: pulse-progress 1.4s ease-in-out infinite alternate;
}

@keyframes pulse-progress {
  from { opacity: 0.7; }
  to { opacity: 1; }
}

.scan-list {
  display: grid;
  gap: 10px;
  padding: 0;
  list-style: none;
}

.scan-list li {
  padding: 12px 14px;
  border: 1px solid #d4e3e0;
  border-radius: 8px;
  background: #f8fbfa;
}

.split-title {
  display: flex;
  justify-content: space-between;
  gap: 24px;
  align-items: start;
}

.segmented {
  display: inline-flex;
  border: 1px solid #c9ddda;
  border-radius: 8px;
  overflow: hidden;
}

.segmented button {
  padding: 10px 16px;
  background: #f5faf9;
  color: #31575a;
  cursor: pointer;
}

.segmented button[data-active="true"] {
  background: #127a75;
  color: white;
}

.cleanup-list {
  display: grid;
  gap: 20px;
  margin: 26px 0;
}

.cleanup-group {
  display: grid;
  gap: 10px;
}

.cleanup-group > h3 {
  margin: 0;
  color: #174247;
}

.cleanup-row {
  display: grid;
  grid-template-columns: 34px 1fr auto;
  gap: 14px;
  align-items: start;
  border: 1px solid #d4e3e0;
  border-radius: 8px;
  padding: 16px;
  background: #ffffff;
}

.cleanup-row[data-disabled="true"] {
  background: #f6f1ee;
}

.cleanup-check {
  padding-top: 4px;
}

.cleanup-title-line {
  display: flex;
  align-items: center;
  gap: 10px;
}

.cleanup-title-line h3 {
  margin: 0;
  font-size: 17px;
}

.path-hint,
.reason,
.warning {
  margin: 6px 0 0;
  font-size: 13px;
}

.path-hint {
  color: #657b7d;
}

.reason {
  color: #31575a;
}

.warning {
  color: #9b4e12;
}

.bytes {
  color: #174247;
  white-space: nowrap;
}

.risk-badge {
  border-radius: 999px;
  padding: 3px 8px;
  font-size: 12px;
  font-weight: 700;
}

.risk-badge[data-risk="recommended"] {
  color: #0d625d;
  background: #dff4f1;
}

.risk-badge[data-risk="optional"] {
  color: #75510b;
  background: #fff2cf;
}

.risk-badge[data-risk="highRisk"] {
  color: #8f2a24;
  background: #ffe1dd;
}

.risk-badge[data-risk="notCleanable"] {
  color: #586367;
  background: #e8ecec;
}

.confirm-list {
  display: grid;
  gap: 10px;
  margin: 24px 0;
}

.confirm-row {
  display: flex;
  justify-content: space-between;
  gap: 20px;
  padding: 14px;
  border: 1px solid #d4e3e0;
  border-radius: 8px;
  background: #f8fbfa;
}

.danger-confirm {
  display: flex;
  gap: 10px;
  align-items: start;
  border: 1px solid #f0b8a9;
  border-radius: 8px;
  padding: 14px;
  background: #fff6f3;
  color: #7d2e24;
}

.error-panel,
.privacy-notice,
.admin-note {
  margin: 18px 0;
  padding: 16px;
  border: 1px solid #d4e3e0;
  border-radius: 8px;
  background: #f8fbfa;
}

.error-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  border-color: #f0b8a9;
  background: #fff6f3;
}

.error-panel h3,
.privacy-notice h3,
.admin-note h3 {
  margin: 0 0 8px;
  font-size: 18px;
}

.error-panel p,
.privacy-notice p,
.admin-note p {
  margin: 0;
}

.privacy-notice label {
  display: flex;
  gap: 10px;
  align-items: center;
  margin-top: 12px;
}

.button-row {
  display: flex;
  gap: 12px;
  margin-top: 24px;
}
```

- [ ] **Step 6: Run frontend tests and build**

Run:

```powershell
npm test
npm run build
```

Expected: tests pass and build exits 0.

- [ ] **Step 7: Run local UI preview**

Run:

```powershell
npm run dev
```

Open `http://localhost:1420`.

Expected:

- Welcome page is visible.
- Scan button transitions to suggestions after mock scan in browser preview.
- Suggestions page can switch between risk and source views.
- High-risk confirmation disables cleanup until checked.
- A failed scan or cleanup call shows the error panel and offers retry.

- [ ] **Step 8: Commit wizard UI**

Run:

```powershell
git add src/App.tsx src/components src/styles/theme.css
git commit -m "feat: build cleanup wizard interface"
```

Expected: commit succeeds.

## Task 12: Implement Privacy Settings And Analytics Payload Sanitization

**Files:**

- Create: `src/components/PrivacyNotice.tsx`
- Create: `src-tauri/src/analytics.rs`
- Create: `src-tauri/tests/analytics_tests.rs`
- Modify: `src/App.tsx`
- Modify: `src/components/WelcomeStep.tsx`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write analytics tests**

Create `src-tauri/tests/analytics_tests.rs`:

```rust
use c_drive_cleaner::analytics::{build_scan_analytics_event, SpaceBucket};

#[test]
fn analytics_event_uses_space_buckets() {
    let event = build_scan_analytics_event("0.1.0", 6_000_000_000, vec!["system".to_string()]);
    assert_eq!(event.freed_space_bucket, SpaceBucket::FiveGbPlus);
}

#[test]
fn analytics_event_does_not_include_paths_or_usernames() {
    let event = build_scan_analytics_event(
        "0.1.0",
        42,
        vec!["C:\\Users\\Administrator\\AppData\\Local\\Temp\\abc.txt".to_string()],
    );
    let encoded = serde_json::to_string(&event).expect("json");
    assert!(!encoded.contains("Administrator"));
    assert!(!encoded.contains("AppData"));
    assert!(!encoded.contains("abc.txt"));
}
```

- [ ] **Step 2: Implement analytics sanitizer**

Create `src-tauri/src/analytics.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SpaceBucket {
    ZeroToOneGb,
    OneToFiveGb,
    FiveGbPlus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsEvent {
    pub app_version: String,
    pub event_name: String,
    pub freed_space_bucket: SpaceBucket,
    pub categories: Vec<String>,
}

pub fn build_scan_analytics_event(app_version: &str, freed_bytes: u64, raw_categories: Vec<String>) -> AnalyticsEvent {
    AnalyticsEvent {
        app_version: app_version.to_string(),
        event_name: "cleanup_completed".to_string(),
        freed_space_bucket: bucket_for_bytes(freed_bytes),
        categories: raw_categories.into_iter().map(sanitize_category).collect(),
    }
}

fn bucket_for_bytes(bytes: u64) -> SpaceBucket {
    let one_gb = 1024_u64.pow(3);
    if bytes < one_gb {
        SpaceBucket::ZeroToOneGb
    } else if bytes < one_gb * 5 {
        SpaceBucket::OneToFiveGb
    } else {
        SpaceBucket::FiveGbPlus
    }
}

fn sanitize_category(input: String) -> String {
    match input.as_str() {
        "system" | "wechat" | "qq" | "workChat" | "cloudDrive" | "installersOldVersions" | "commonSoftware" => input,
        _ => "other".to_string(),
    }
}
```

- [ ] **Step 3: Create privacy notice UI**

Create `src/components/PrivacyNotice.tsx`:

```tsx
export function PrivacyNotice({
  enabled,
  onEnabledChange,
}: {
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
}) {
  return (
    <section className="privacy-notice">
      <h3>匿名统计</h3>
      <p>默认开启匿名统计，只上传规则类别、空间区间和错误类型，不上传完整路径、文件名、用户名或文件内容。</p>
      <label>
        <input checked={enabled} onChange={(event) => onEnabledChange(event.target.checked)} type="checkbox" />
        允许匿名统计帮助改进软件
      </label>
    </section>
  );
}
```

- [ ] **Step 4: Wire privacy notice into the welcome page**

Modify `src/components/WelcomeStep.tsx` so the privacy notice is visible on first launch:

```tsx
import { PrivacyNotice } from "./PrivacyNotice";

export function WelcomeStep({
  onStart,
  analyticsEnabled,
  onAnalyticsEnabledChange,
}: {
  onStart: () => void;
  analyticsEnabled: boolean;
  onAnalyticsEnabledChange: (enabled: boolean) => void;
}) {
  return (
    <div className="step-content">
      <p className="eyebrow">开始前</p>
      <h2>先扫描，不会直接删除任何文件</h2>
      <p>软件会检查 C 盘里的临时文件、缓存、安装包、常见软件数据和配置引用风险。</p>
      <div className="assurance-grid">
        <div>默认只勾选推荐清理项</div>
        <div>高风险项目需要二次确认</div>
        <div>被配置引用的目录不会清理</div>
      </div>
      <PrivacyNotice enabled={analyticsEnabled} onEnabledChange={onAnalyticsEnabledChange} />
      <button className="primary-button" onClick={onStart}>
        开始扫描 C 盘
      </button>
    </div>
  );
}
```

Modify `src/App.tsx` to persist the analytics toggle locally:

```tsx
const [analyticsEnabled, setAnalyticsEnabled] = useState(() => localStorage.getItem("analyticsEnabled") !== "false");

function updateAnalyticsEnabled(enabled: boolean) {
  setAnalyticsEnabled(enabled);
  localStorage.setItem("analyticsEnabled", String(enabled));
}
```

Then update the welcome step render:

```tsx
{step === "welcome" && (
  <WelcomeStep
    analyticsEnabled={analyticsEnabled}
    onAnalyticsEnabledChange={updateAnalyticsEnabled}
    onStart={startScan}
  />
)}
```

- [ ] **Step 5: Export analytics module**

Modify `src-tauri/src/lib.rs` and add `pub mod analytics;` at the top with the existing module exports.

- [ ] **Step 6: Run analytics tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml analytics -- --nocapture
npm run build
```

Expected: tests pass and frontend build exits 0.

- [ ] **Step 7: Commit privacy and analytics**

Run:

```powershell
git add src/App.tsx src/components/PrivacyNotice.tsx src/components/WelcomeStep.tsx src-tauri/src/analytics.rs src-tauri/tests/analytics_tests.rs src-tauri/src/lib.rs
git commit -m "feat: add privacy-safe analytics payloads"
```

Expected: commit succeeds.

## Task 13: Add Administrator Mode Entry Point

**Files:**

- Create: `src-tauri/src/admin.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/components/WelcomeStep.tsx`

- [ ] **Step 1: Implement admin mode command descriptor**

Create `src-tauri/src/admin.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminCleanupCapability {
    pub available: bool,
    pub title: String,
    pub description: String,
    pub supported_items: Vec<String>,
}

pub fn lightweight_admin_capability() -> AdminCleanupCapability {
    AdminCleanupCapability {
        available: false,
        title: "系统轻量清理（V0.2 计划）".to_string(),
        description: "V0.1 只展示能力说明，不执行提权清理；后续版本可在用户主动授权后清理 Windows 临时目录、Windows 更新下载缓存和安全系统日志。".to_string(),
        supported_items: vec![
            "Windows 临时目录".to_string(),
            "Windows 更新下载缓存".to_string(),
            "系统日志".to_string(),
        ],
    }
}
```

- [ ] **Step 2: Export admin command**

Modify `src-tauri/src/lib.rs`:

```rust
pub mod admin;
pub mod analytics;
pub mod cleanup;
pub mod config_refs;
pub mod drive;
pub mod errors;
pub mod fixtures;
pub mod models;
pub mod paths;
pub mod processes;
pub mod rules;
pub mod scan;
pub mod size;

use admin::AdminCleanupCapability;
use models::{CleanupResult, CleanupSelection, ScanReport};
use paths::ScanRoots;

#[tauri::command]
fn ping() -> &'static str {
    "ok"
}

#[tauri::command]
fn get_admin_cleanup_capability() -> AdminCleanupCapability {
    admin::lightweight_admin_capability()
}

#[tauri::command]
fn scan_c_drive() -> Result<ScanReport, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    Ok(scan::scan_with_roots(&roots, drive_summary))
}

#[tauri::command]
fn execute_cleanup(selection: CleanupSelection) -> Result<CleanupResult, String> {
    let roots = ScanRoots::from_current_user().map_err(|error| error.to_string())?;
    let drive_summary = drive::c_drive_summary().map_err(|error| error.to_string())?;
    let report = scan::scan_with_roots(&roots, drive_summary);
    cleanup::execute_selected_cleanup(&selection, &report.items, &roots)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping, get_admin_cleanup_capability, scan_c_drive, execute_cleanup])
        .run(tauri::generate_context!())
        .expect("failed to run C Drive Cleaner");
}
```

- [ ] **Step 3: Add admin entry copy to welcome page**

Modify `src/components/WelcomeStep.tsx`:

```tsx
import { PrivacyNotice } from "./PrivacyNotice";

export function WelcomeStep({
  onStart,
  analyticsEnabled,
  onAnalyticsEnabledChange,
}: {
  onStart: () => void;
  analyticsEnabled: boolean;
  onAnalyticsEnabledChange: (enabled: boolean) => void;
}) {
  return (
    <div className="step-content">
      <p className="eyebrow">开始前</p>
      <h2>先扫描，不会直接删除任何文件</h2>
      <p>软件会检查 C 盘里的临时文件、缓存、安装包、常见软件数据和配置引用风险。</p>
      <div className="assurance-grid">
        <div>默认只勾选推荐清理项</div>
        <div>高风险项目需要二次确认</div>
        <div>被配置引用的目录不会清理</div>
      </div>
      <PrivacyNotice enabled={analyticsEnabled} onEnabledChange={onAnalyticsEnabledChange} />
      <section className="admin-note">
        <h3>系统轻量清理</h3>
        <p>普通模式不需要管理员权限。V0.1 只展示管理员清理能力说明，不执行提权清理；后续版本会在你主动选择后再请求管理员权限。</p>
      </section>
      <button className="primary-button" onClick={onStart}>
        开始扫描 C 盘
      </button>
    </div>
  );
}
```

- [ ] **Step 4: Verify admin module compile**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

Expected: both commands exit 0.

- [ ] **Step 5: Commit admin mode entry**

Run:

```powershell
git add src-tauri/src/admin.rs src-tauri/src/lib.rs src/components/WelcomeStep.tsx
git commit -m "feat: describe optional administrator cleanup mode"
```

Expected: commit succeeds.

## Task 14: Build And Packaging Verification

**Files:**

- Modify: `README.md`

- [ ] **Step 1: Update README with development commands**

Modify `README.md`:

````markdown
# C Drive Cleaner

Local Windows desktop app for safely scanning and cleaning the C drive.

## Current Outputs

- Product/spec design: `docs/superpowers/specs/2026-06-11-c-drive-cleaner-design.md`
- Implementation plan: `docs/superpowers/plans/2026-06-11-c-drive-cleaner-implementation.md`

## Development

Install dependencies:

```powershell
npm install
```

Run frontend tests:

```powershell
npm test
```

Run Rust tests:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Run browser preview:

```powershell
npm run dev
```

Run desktop app:

```powershell
npm run tauri:dev
```

Build desktop installer:

```powershell
npm run tauri:build
```

## Safety Notes

The Rust backend owns all filesystem decisions. The frontend must never construct cleanup paths or decide that a path is safe to delete.
````

- [ ] **Step 2: Run full verification**

Run:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected:

- Frontend tests pass.
- Frontend production build exits 0.
- Rust tests pass.

- [ ] **Step 3: Run desktop dev app**

Run:

```powershell
npm run tauri:dev
```

Expected:

- Tauri window opens.
- Welcome page renders.
- Scan flow reaches suggestions page.
- Suggestions page has `按风险` and `按来源` controls.
- Confirm page blocks high-risk cleanup until the second confirmation checkbox is selected.

- [ ] **Step 4: Build installer**

Run:

```powershell
npm run tauri:build
```

Expected:

- Command exits 0.
- Installer artifact is created under `src-tauri\target\release\bundle\nsis`.
- Portable executable is created under `src-tauri\target\release`.

- [ ] **Step 5: Commit documentation and packaging verification**

Run:

```powershell
git add README.md
git commit -m "docs: add development and packaging commands"
```

Expected: commit succeeds.

## Final Verification Checklist

Run this from `H:\临时对话\c-drive-cleaner` before claiming the implementation is complete:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

Required evidence:

- `npm test` exits 0.
- `npm run build` exits 0.
- `cargo test` exits 0.
- `npm run tauri:build` exits 0 and creates NSIS output.

Manual verification:

- The app starts without asking for administrator permission.
- The welcome page explains scan-first behavior.
- Scan result displays recommended, optional, high-risk, and not-cleanable items.
- `按风险` and `按来源` views show the same items with synchronized selection.
- Not-cleanable items cannot be selected.
- High-risk items require second confirmation.
- Browser preview does not delete files.
- Desktop cleanup command revalidates before deletion.
- Config-referenced paths are classified as not cleanable.
- Process-used paths are classified as not cleanable.

