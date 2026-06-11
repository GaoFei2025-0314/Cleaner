# C Drive Cleaner V0.1 中文实现计划

> **给执行 Agent 的要求：** 实施本计划时必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans`，逐任务执行。任务使用 checkbox (`- [ ]`) 跟踪。

**目标：** 构建 V0.1 版本的 Windows 本地桌面软件，帮助普通用户安全扫描并清理 C 盘，按风险和来源展示清理建议，用户确认后才删除已选项目。

**架构：** 使用 Tauri 2 作为桌面壳，React + TypeScript 构建向导式界面，Rust 后端负责所有文件系统扫描、安全判断、删除执行、管理员模式入口和隐私安全的统计事件构造。前端只展示后端结果和提交用户选择，不直接判断路径是否安全。

**技术栈：** Tauri 2、React、TypeScript、Vite、Rust、Serde、sysinfo、Vitest、Testing Library、Cargo tests、Tauri NSIS 打包。

---

## 与英文计划的关系

本文件是中文执行说明版，便于产品和开发沟通。英文计划文件仍保留为代码级执行版：

`docs/superpowers/plans/2026-06-11-c-drive-cleaner-implementation.md`

执行时以两份文件共同为准：

- 中文版用于理解任务目标、顺序、验收点。
- 英文版保留完整代码片段、精确文件内容和命令细节。
- 如果两份文件出现冲突，以英文代码级计划中的具体代码为准，再回补中文计划。

## 范围检查

本项目包含多个子系统：

- 桌面 UI
- 扫描规则
- 安全校验
- 清理执行器
- 可选管理员模式
- 匿名统计
- 打包发布

这些子系统共享同一套领域模型，不能完全拆成独立产品，因此本计划保持为一个实现轨道，但每个任务都必须能单独测试和提交。

第一条可用主链路必须覆盖：

- 普通模式扫描 C 盘用户目录。
- 按风险和来源组织扫描结果。
- 检测配置文件引用，避免误删工具运行入口。
- 检测运行进程占用，避免强杀或误删。
- 用户确认后直接删除已选项目。
- 高风险项目二次确认。
- 匿名统计只上传安全字段。
- V0.1 不提供聊天软件根目录删除；微信/QQ 根目录只能解释为不可清理。
- V0.1 不执行提权清理；管理员模式只提供入口说明和后续能力展示。

## 目标文件结构

项目根目录：

`H:\临时对话\c-drive-cleaner`

目标结构：

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
    components/ErrorPanel.tsx
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

职责边界：

- `src-tauri/src/models.rs`：后端与前端共享的数据传输模型。
- `src-tauri/src/rules.rs`：内置清理规则。
- `src-tauri/src/scan.rs`：执行扫描并生成 `ScanReport`。
- `src-tauri/src/config_refs.rs`：检测候选路径是否被配置文件引用。
- `src-tauri/src/processes.rs`：检测候选路径是否被运行进程使用。
- `src-tauri/src/cleanup.rs`：清理前复查并执行删除。
- `src-tauri/src/admin.rs`：可选管理员模式能力描述。
- `src-tauri/src/analytics.rs`：生成隐私安全的匿名统计事件。
- `src/domain/*.ts`：前端选择、分组和展示用的纯逻辑。
- `src/components/*.tsx`：向导式界面组件。

## Task 1：初始化桌面应用骨架

**目标：** 创建 Tauri + React + TypeScript 项目骨架，能通过前端构建和 Rust 编译。

**涉及文件：**

- 新建：`package.json`
- 新建：`index.html`
- 新建：`vite.config.ts`
- 新建：`tsconfig.json`
- 新建：`tsconfig.node.json`
- 新建：`vitest.config.ts`
- 新建：`src/main.tsx`
- 新建：`src/App.tsx`
- 新建：`src/styles/theme.css`
- 新建：`src-tauri/Cargo.toml`
- 新建：`src-tauri/tauri.conf.json`
- 新建：`src-tauri/build.rs`
- 新建：`src-tauri/src/main.rs`
- 新建：`src-tauri/src/lib.rs`

执行步骤：

- [ ] 在 `H:\临时对话\c-drive-cleaner` 初始化 git。
- [ ] 创建前端 `package.json`，包含 `dev`、`build`、`test`、`tauri:dev`、`tauri:build` 脚本。
- [ ] 创建 Vite、TypeScript、Vitest 配置。
- [ ] 创建最小 React 入口页面。
- [ ] 创建 Tauri Rust 骨架和 `ping` 命令。
- [ ] 运行依赖安装和验证命令。

验证命令：

```powershell
npm install
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

期望结果：

- `npm install` 生成 `package-lock.json`。
- `npm run build` 成功生成 `dist/`。
- `cargo test` 退出码为 0。

提交：

```powershell
git add .
git commit -m "chore: initialize tauri desktop app"
```

## Task 2：定义前后端共享领域模型

**目标：** 定义扫描结果、清理项、风险等级、来源类别、清理选择和清理结果的数据结构。

**涉及文件：**

- 新建：`src/domain/models.ts`
- 新建：`src-tauri/src/models.rs`
- 修改：`src-tauri/src/lib.rs`

关键模型：

- `RiskLevel`
  - `recommended`
  - `optional`
  - `highRisk`
  - `notCleanable`
- `SourceCategory`
  - `system`
  - `commonSoftware`
  - `wechat`
  - `qq`
  - `workChat`
  - `cloudDrive`
  - `installersOldVersions`
  - `otherLarge`
- `CleanupAction`
  - `directDelete`
  - `requiresAdmin`
  - `explainOnly`
  - `blockedByProcess`
  - `blockedByConfigReference`
- `ScanItem`
- `ScanReport`
- `CleanupSelection`
- `CleanupResult`

执行步骤：

- [ ] 在 TypeScript 中创建对应接口和联合类型。
- [ ] 在 Rust 中创建对应 `enum` 和 `struct`，使用 Serde 的 `camelCase` 序列化。
- [ ] 在 `lib.rs` 中导出 `models` 模块。
- [ ] 运行前后端编译。

验证命令：

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

提交：

```powershell
git add src/domain/models.ts src-tauri/src/models.rs src-tauri/src/lib.rs
git commit -m "feat: add shared cleanup domain models"
```

## Task 3：实现前端选择和分组逻辑

**目标：** 前端能根据扫描结果默认勾选可直接清理的推荐项，阻止选择不可清理项和 V0.1 不执行的管理员项，并支持按风险/按来源两种视图分组。

**涉及文件：**

- 新建：`src/domain/selection.ts`
- 新建：`src/domain/grouping.ts`
- 新建：`src/__tests__/selection.test.ts`
- 新建：`src/__tests__/grouping.test.ts`

测试要求：

- 默认只选中 `defaultSelected=true` 且 `cleanupAction=directDelete` 的项目。
- `notCleanable` 项目无法被选中。
- `requiresAdmin` 项目在 V0.1 无法被默认选中，也不能手动加入本轮清理。
- 高风险项目可以手动选中。
- 只要已选项目里包含 `highRisk`，确认页必须要求二次确认。
- `groupByRisk` 和 `groupBySource` 分组正确。

执行步骤：

- [ ] 先写 `selection.test.ts`。
- [ ] 先写 `grouping.test.ts`。
- [ ] 运行 `npm test`，确认因为实现不存在而失败。
- [ ] 实现 `selection.ts`。
- [ ] 实现 `grouping.ts`。
- [ ] 再次运行测试，确认通过。

验证命令：

```powershell
npm test
```

提交：

```powershell
git add src/domain src/__tests__
git commit -m "feat: add cleanup selection and grouping logic"
```

## Task 4：实现 Rust 内置规则注册表

**目标：** 后端维护白名单式清理规则，避免“看到大目录就删”的危险逻辑。

**涉及文件：**

- 新建：`src-tauri/src/rules.rs`
- 新建：`src-tauri/src/fixtures.rs`
- 新建：`src-tauri/tests/rule_registry_tests.rs`
- 修改：`src-tauri/src/lib.rs`

首批规则：

- `user-temp`：用户临时文件，推荐清理。
- `windows-temp`：Windows 临时文件，需要管理员权限；V0.1 只展示为需要管理员能力，不执行提权删除。
- `windows-update-download`：Windows 更新下载缓存，需要管理员权限；V0.1 只展示为需要管理员能力，不执行提权删除。
- `wechat-data-root`：微信数据根目录，不可清理，只解释风险。
- `qq-data-root`：QQ 数据根目录，不可清理，只解释风险。
- `vscode-cached-vsix`：VS Code 扩展安装包缓存，推荐清理。

测试要求：

- `user-temp` 是推荐清理项。
- 高风险规则绝不能默认勾选。
- 不可清理规则绝不能默认勾选。
- `requiresAdmin` 规则在 V0.1 绝不能默认勾选。
- 微信/QQ 根目录必须是 `notCleanable + explainOnly`，不得作为 `directDelete` 规则出现。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml rule_registry -- --nocapture
```

提交：

```powershell
git add src-tauri/src/rules.rs src-tauri/src/fixtures.rs src-tauri/tests/rule_registry_tests.rs src-tauri/src/lib.rs
git commit -m "feat: add bundled cleanup rules"
```

## Task 5：实现路径解析、C 盘容量读取和目录大小计算

**目标：** 后端能安全解析规则路径、限制路径范围、读取真实 C 盘容量，并计算候选项大小。

**涉及文件：**

- 修改：`src-tauri/Cargo.toml`
- 新建：`src-tauri/src/errors.rs`
- 新建：`src-tauri/src/drive.rs`
- 新建：`src-tauri/src/paths.rs`
- 新建：`src-tauri/src/size.rs`
- 修改：`src-tauri/src/lib.rs`

实现要点：

- `drive.rs` 使用 `sysinfo::Disks` 读取真实 C 盘总量和剩余空间。
- `paths.rs` 提供 `ScanRoots`，包含：
  - `c_drive`
  - `user_profile`
  - `local_app_data`
  - `windows_dir`
- `resolve_rule_path` 根据 `RuleScope` 转换成实际路径。
- `ensure_under_root` 确保路径不越界。
- `size.rs` 用 `walkdir` 计算文件或目录总大小。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

提交：

```powershell
git add src-tauri/Cargo.toml src-tauri/src/errors.rs src-tauri/src/drive.rs src-tauri/src/paths.rs src-tauri/src/size.rs src-tauri/src/lib.rs
git commit -m "feat: add path and size utilities"
```

## Task 6：实现配置引用保护

**目标：** 防止误删“看起来像缓存或旧版本，但被配置文件当作运行入口”的目录。

**涉及文件：**

- 新建：`src-tauri/src/config_refs.rs`
- 新建：`src-tauri/tests/config_refs_tests.rs`
- 修改：`src-tauri/src/lib.rs`

背景案例：

- `npm-cache/_npx/.../exa-mcp-server` 看起来像缓存，但被 MCP 配置引用。
- `highagency.pencildev-0.6.51` 看起来像旧扩展，但被工具配置引用。

扫描范围：

- 用户根目录下的 `.claude.json`
- 用户根目录下的 `.codex.json`
- 用户目录下的 `.codex`
- `.claude`
- `.cursor`
- `.vscode`
- `.trae`

只读取常见纯文本配置文件：

- `json`
- `jsonl`
- `toml`
- `yaml`
- `yml`
- `ini`
- `txt`
- `config`
- `conf`

安全要求：

- 不读取二进制内容。
- 不上传配置内容。
- 只返回“用户配置中存在引用”这类解释信息。
- 被引用路径必须分类为 `notCleanable`。
- 路径匹配前必须同时归一化 `/`、`\` 和 JSON 转义后的 `\\`，避免漏掉配置里的 Windows 路径。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml config_refs -- --nocapture
```

提交：

```powershell
git add src-tauri/src/config_refs.rs src-tauri/tests/config_refs_tests.rs src-tauri/src/lib.rs
git commit -m "feat: detect config references before cleanup"
```

## Task 7：实现进程占用保护

**目标：** 如果候选目录正在被运行进程使用，则本轮不清理，并提示用户关闭相关软件后重试。

**涉及文件：**

- 新建：`src-tauri/src/processes.rs`
- 修改：`src-tauri/src/lib.rs`

实现要点：

- 使用 `sysinfo` 枚举运行进程。
- 检查进程可执行文件路径是否位于候选目录下。
- 检查命令行参数是否包含候选目录。
- 只返回进程名，不展示长路径。
- 不自动杀进程。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

提交：

```powershell
git add src-tauri/src/processes.rs src-tauri/src/lib.rs
git commit -m "feat: detect running processes using cleanup paths"
```

## Task 8：实现扫描报告生成

**目标：** 后端把内置规则、路径解析、容量统计、配置引用检查和进程占用检查整合成 `ScanReport`。

**涉及文件：**

- 新建：`src-tauri/src/scan.rs`
- 修改：`src-tauri/src/lib.rs`

实现要点：

- `scan_with_roots` 遍历内置规则。
- 每个规则解析出实际路径。
- 计算候选项大小。
- 候选路径不存在且大小为 0 时跳过。
- 检测配置引用。
- 检测运行进程占用。
- 如果被配置引用，风险强制改为 `notCleanable`，动作为 `blockedByConfigReference`。
- 如果被进程占用，风险强制改为 `notCleanable`，动作为 `blockedByProcess`。
- Tauri 命令 `scan_c_drive` 使用真实 C 盘容量，不使用伪值。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

提交：

```powershell
git add src-tauri/src/scan.rs src-tauri/src/lib.rs
git commit -m "feat: generate cleanup scan reports"
```

## Task 9：实现清理执行器和执行前复查

**目标：** 用户确认后，后端重新扫描并执行已选项目；删除前必须再次校验风险和动作。

**涉及文件：**

- 新建：`src-tauri/src/cleanup.rs`
- 新建：`src-tauri/tests/cleanup_tests.rs`
- 修改：`src-tauri/src/lib.rs`

测试要求：

- `delete_path_contents` 删除目录内容但保留父目录。
- 高风险项目未二次确认时必须拒绝执行。

执行逻辑：

- `execute_cleanup` 先重新执行 `scan_c_drive`，防止扫描后状态变化。
- 校验高风险项目是否已经二次确认。
- 对 `notCleanable` 项目返回 `skipped`。
- 对非 `directDelete` 项目返回 `skipped`。
- 缺少路径信息返回 `failed`。
- 每个已选项目都必须重新匹配内置规则，找不到规则则返回 `failed`。
- 删除前必须重新解析规则路径，并确认扫描路径与规则路径完全一致。
- 删除前必须调用 `ensure_under_root`，确认路径仍在规则允许的根目录下。
- 删除前必须重新执行配置引用检查；若仍被引用，返回 `skipped`。
- 删除前必须重新执行进程占用检查；若仍被使用，返回 `skipped`。
- 删除前必须检查 `min_age_minutes`，最近仍在变化的目录不清理。
- 删除时必须尊重规则中的 `delete_contents_only`，该保留父目录时不能删掉父目录。
- 返回逐项结果和实际释放空间。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml cleanup -- --nocapture
```

提交：

```powershell
git add src-tauri/src/cleanup.rs src-tauri/tests/cleanup_tests.rs src-tauri/src/lib.rs
git commit -m "feat: add cleanup executor safeguards"
```

## Task 10：实现 Tauri API 桥接和浏览器预览 mock

**目标：** 前端通过统一 API 调用后端；浏览器预览模式使用 mock 数据，不触碰真实文件。

**涉及文件：**

- 新建：`src/services/tauriApi.ts`
- 新建：`src/services/mockReport.ts`

实现要点：

- `scanCDrive()`：
  - Tauri 环境调用 `scan_c_drive`。
  - 浏览器预览环境返回 `mockReport`。
- `executeCleanup()`：
  - Tauri 环境调用 `execute_cleanup`。
  - 浏览器预览环境只模拟结果，不删除文件。

验证命令：

```powershell
npm run build
```

提交：

```powershell
git add src/services
git commit -m "feat: add frontend tauri api bridge"
```

## Task 11：实现向导式前端 UI

**目标：** 做出面向普通用户的向导式 C 盘清理界面，支持扫描、建议、确认、清理、结果五步。

**涉及文件：**

- 新建：`src/components/AppShell.tsx`
- 新建：`src/components/StepIndicator.tsx`
- 新建：`src/components/WelcomeStep.tsx`
- 新建：`src/components/ScanStep.tsx`
- 新建：`src/components/SuggestionsStep.tsx`
- 新建：`src/components/ConfirmStep.tsx`
- 新建：`src/components/CleanStep.tsx`
- 新建：`src/components/ResultStep.tsx`
- 新建：`src/components/ErrorPanel.tsx`
- 新建：`src/components/RiskBadge.tsx`
- 新建：`src/components/CleanupItemRow.tsx`
- 修改：`src/App.tsx`
- 修改：`src/styles/theme.css`

界面要求：

- 左侧固定流程和产品说明。
- 右侧展示当前步骤内容。
- 顶部或侧边显示流程：扫描、建议、确认、清理、结果。
- 建议页提供分段控件：
  - `按风险`
  - `按来源`
- 两种视图共享选择状态。
- 不可清理项 checkbox 禁用。
- 高风险项被选中后，确认页必须展示二次确认 checkbox。
- `scanCDrive()` 或 `executeCleanup()` 失败时必须展示错误面板，提供“先不处理”和“重试”。
- 不做营销页，不做夸张警报风格。

验证命令：

```powershell
npm test
npm run build
npm run dev
```

手动验收：

- 欢迎页可见。
- 点击开始扫描后进入建议页。
- 建议页可以切换 `按风险` / `按来源`。
- 勾选高风险项目后，确认页必须勾选二次确认才能继续。
- 模拟扫描或清理失败时，页面展示错误面板且不会删除新项目。

提交：

```powershell
git add src/App.tsx src/components src/styles/theme.css
git commit -m "feat: build cleanup wizard interface"
```

## Task 12：实现隐私设置和匿名统计事件净化

**目标：** 支持默认开启匿名统计，但只构造隐私安全的统计事件，不包含路径、文件名、用户名或配置内容。

**涉及文件：**

- 新建：`src/components/PrivacyNotice.tsx`
- 新建：`src-tauri/src/analytics.rs`
- 新建：`src-tauri/tests/analytics_tests.rs`
- 修改：`src/App.tsx`
- 修改：`src/components/WelcomeStep.tsx`
- 修改：`src-tauri/src/lib.rs`

允许统计：

- 软件版本
- Windows 大版本
- 扫描耗时区间
- 命中规则类别
- 释放空间区间
- 错误类别
- 用户是否关闭统计

禁止统计：

- 完整路径
- 文件名
- 用户名
- 配置文件内容
- 聊天记录
- 图片、视频、文档内容
- 软件账号信息
- 稳定硬件标识

测试要求：

- 释放空间用区间表示。
- 任意原始路径输入都只能被归类为安全类别或 `other`。
- 序列化后的事件中不能包含用户名、`AppData`、文件名等敏感信息。
- 欢迎页必须展示匿名统计说明。
- `analyticsEnabled` 必须写入 `localStorage`，用户关闭后下次启动保持关闭。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml analytics -- --nocapture
npm run build
```

提交：

```powershell
git add src/App.tsx src/components/PrivacyNotice.tsx src/components/WelcomeStep.tsx src-tauri/src/analytics.rs src-tauri/tests/analytics_tests.rs src-tauri/src/lib.rs
git commit -m "feat: add privacy-safe analytics payloads"
```

## Task 13：添加可选管理员模式入口

**目标：** 普通启动不申请管理员权限；V0.1 只展示系统轻量清理能力说明，不执行提权清理。

**涉及文件：**

- 新建：`src-tauri/src/admin.rs`
- 修改：`src-tauri/src/lib.rs`
- 修改：`src/components/WelcomeStep.tsx`

V0.1 展示的后续管理员模式能力：

- Windows 临时目录。
- Windows 更新下载缓存。
- 系统日志。

V0.1 明确不做：

- 请求 UAC 或重新以管理员身份启动。
- 执行 `requiresAdmin` 规则的实际删除。
- DISM 组件清理。
- 系统还原点清理。
- 手动清理 WinSxS。
- 修改受保护系统组件。

实现要求：

- `get_admin_cleanup_capability` 可以返回能力描述。
- 返回值必须明确 `available=false`，表示当前版本不可执行提权清理。
- 欢迎页文案必须写清楚“V0.1 只展示说明，后续版本再加入主动授权后的管理员清理”。

验证命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```

提交：

```powershell
git add src-tauri/src/admin.rs src-tauri/src/lib.rs src/components/WelcomeStep.tsx
git commit -m "feat: describe optional administrator cleanup mode"
```

## Task 14：构建和打包验证

**目标：** 更新 README，并验证测试、构建、桌面启动和安装包产物。

**涉及文件：**

- 修改：`README.md`

README 需要包含：

- spec 路径。
- implementation plan 路径。
- 依赖安装命令。
- 前端测试命令。
- Rust 测试命令。
- 浏览器预览命令。
- Tauri 桌面运行命令。
- Tauri 打包命令。
- 安全说明：前端不得构造清理路径，文件系统决策只能由 Rust 后端执行。

完整验证命令：

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

期望结果：

- `npm test` 退出码为 0。
- `npm run build` 退出码为 0。
- `cargo test` 退出码为 0。
- `npm run tauri:build` 退出码为 0。
- NSIS 安装包生成在 `src-tauri\target\release\bundle\nsis`。
- 便携 EXE 生成在 `src-tauri\target\release`。

提交：

```powershell
git add README.md
git commit -m "docs: add development and packaging commands"
```

## 最终验收清单

最终声称完成前，必须在 `H:\临时对话\c-drive-cleaner` 运行：

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri:build
```

必须拿到的证据：

- 前端测试通过。
- 前端生产构建通过。
- Rust 测试通过。
- Tauri 打包通过，并生成 NSIS 产物。

手动验收：

- 应用启动时不请求管理员权限。
- 欢迎页明确说明“先扫描，不直接删除”。
- 扫描结果展示推荐、可选、高风险、不可清理四类。
- `按风险` 与 `按来源` 展示同一批结果，勾选状态同步。
- 不可清理项无法勾选。
- 高风险项必须二次确认。
- 微信/QQ 根目录展示为不可清理，不能被勾选。
- 管理员模式只展示说明，不请求 UAC，不执行提权删除。
- 浏览器预览不会删除文件。
- 桌面清理命令在删除前重新扫描并复查。
- 被配置引用的路径归类为不可清理。
- 被进程使用的路径归类为不可清理。

## 建议执行方式

推荐使用 **Subagent-Driven**：

- 每个任务派发一个新 subagent。
- 每个任务完成后主会话做 review。
- 发现计划问题时先修计划，再继续实现。

如果要在当前会话内执行，则使用 **Inline Execution**：

- 按任务批量执行。
- 每个阶段有 checkpoint。
- 每次声称完成前必须运行对应验证命令。
