# C Drive Cleaner Desktop App Design - V0.1

Date: 2026-06-11

Version: V0.1

## Product Summary

This product is a local Windows desktop app for safely cleaning the C drive. V0.1 targets ordinary personal users who do not understand directories such as AppData, Windows cache folders, editor extension folders, or tool runtime caches.

The primary goal is not to delete as much as possible. The primary goal is to help users understand what is taking space, what can be cleaned, why it is safe or risky, and require explicit authorization before deletion.

## V0.1 Scope

- Platform: Windows desktop only.
- Disk scope: C drive only.
- Default mode: normal user mode.
- Optional mode: administrator-mode entry and capability explanation only; actual elevated cleanup execution is deferred to V0.2.
- Primary flow: wizard style.
- Distribution: standard installer and portable single EXE.
- Cleanup rules: bundled with app releases, no online hot updates.
- History: no persistent cleanup history after app close.
- Analytics: anonymous statistics enabled by default, disclosed on first launch, user can disable it.

Out of scope for the first version:

- Scanning or cleaning non-C drives.
- DISM component store cleanup.
- WinSxS manual cleanup.
- System restore point cleanup.
- Automatic repair of user configuration files.
- Cloud-assisted file classification.
- Online cleanup rule hot updates.
- Saving full local cleanup history.
- Broad deletion of chat application root directories.
- Actual elevated administrator cleanup execution.

## Design Direction

The interface should feel like a calm, trustworthy system health tool. It should avoid alarmist security-software styling and avoid marketing-page visuals.

Visual principles:

- Use a restrained blue-green or cyan primary color for safe, controlled actions.
- Use orange for items that need attention or confirmation.
- Use red only for dangerous or blocked items.
- Use a clear wizard stepper: Scan, Suggestions, Confirm, Clean, Result.
- Show conclusions first; hide technical paths and detailed explanations behind expand controls.
- Use familiar controls: checkboxes for cleanup selection, segmented controls for view switching, progress bars for scanning and cleanup, and icon buttons only where the meaning is familiar.
- Write user-facing copy in plain language, not system jargon.

Example copy:

- "User temporary files: temporary material left by apps. Usually safe to delete."
- "Referenced by a tool configuration: this folder looks like cache, but a tool still starts from here. It will not be cleaned."
- "Close the related app and scan again."

## User Flow

### 1. Welcome And Permission Page

The app shows:

- C drive total size.
- Current free space.
- A plain-language explanation of what the app will scan.
- A primary button: "Start C Drive Scan".
- A secondary entry for "Lightweight System Cleanup", explaining that V0.1 only shows the capability and a later version may request administrator permission.

The app must not request administrator permission at startup.

### 2. Scan Page

The app scans supported locations and shows progress by category:

- Temporary files and download cache.
- Windows and user logs.
- Installer and update caches.
- Common software cache usage.
- Configuration reference and process usage checks.

The scan should be cancellable. Cancellation returns partial results only if the app can clearly mark them as partial.

### 3. Suggestions Page

The suggestions page supports two synchronized views:

- By risk.
- By source.

Selection state is shared between both views. Switching views must not duplicate items or double-count space.

The default view is "By risk" because ordinary users first need to know what is safe.

### 4. Confirm Page

The app summarizes:

- Selected items.
- Estimated space to be freed.
- Items that will not be touched.
- High-risk items selected by the user.

If any high-risk item is selected, the page requires an additional confirmation step and lists those items separately.

### 5. Cleaning Page

The app executes each cleanup item independently.

Before deleting each item, it reruns safety checks:

- Path is still inside the expected rule scope.
- The item is still allowed by its rule.
- The item is not process-locked in a way that changes the risk.
- The item is not referenced by known configuration files.

Failure of one item must not stop other unrelated items.

### 6. Result Page

The result page shows:

- Actual freed space.
- Successful items.
- Failed items and reasons.
- Skipped items and reasons.
- Suggested next steps.

The app does not save this result after closing.

## Result Organization

### By Risk

The risk view has four sections:

1. Recommended cleanup
2. Optional cleanup
3. High-risk cleanup
4. Not cleanable

### By Source

The source view has these sections:

1. System cleanup
2. Common software
3. WeChat
4. QQ
5. Lark, DingTalk, and WeCom
6. Cloud drives and sync tools
7. Installers and old versions
8. Other large suspicious items

Each source item still shows a risk label: Recommended, Optional, High Risk, or Not Cleanable.

## Risk Model

### Recommended Cleanup

Recommended items that can be cleaned in normal mode are selected by default. Items that require administrator permission are not selected by default in V0.1 because elevated cleanup execution is deferred.

This group includes low-risk items and middle-risk items that the app can confidently identify as safe:

- User temporary files.
- App temporary files.
- Logs that are not currently active.
- Update download caches.
- Installer caches that are clearly no longer needed.
- Old installer packages.
- Clearly regenerable caches.
- Old software versions that are not running and not referenced by config.

Recommended items are deleted directly after the user confirms cleanup.

### Optional Cleanup

Optional items are not selected by default.

This group includes larger caches that are likely safe but may affect convenience:

- Browser caches.
- Office software caches.
- Media app caches.
- Cloud drive local caches.
- App image and video caches.
- Offline document caches.

The app must explain likely impact before the user selects these items.

### High-Risk Cleanup

High-risk items are not selected by default. The user may manually select them, but the app requires a second confirmation before deleting them.

In V0.1, the product implements the high-risk selection and second-confirmation framework, but bundled rules must not offer broad root-directory deletion for chat applications. A high-risk item can be directly deleted only when the rule targets a precise, well-understood subdirectory. Broad chat roots are classified as Not Cleanable until more granular classifiers are added in a later version.

This group includes:

- WeChat chat files, images, videos, and downloads.
- QQ chat files, images, videos, group files, and downloads.
- Local cloud drive files that may not be fully synced.
- Offline data.
- Chat history databases.
- Large user-created files found under app data folders.

High-risk items can be directly deleted only after manual selection, second confirmation, and precise rule targeting.

### Not Cleanable

Not cleanable items cannot be selected.

This group includes:

- Items currently used by running processes.
- Items referenced by known configuration files.
- System critical directories.
- Unknown directories whose purpose cannot be determined.
- Paths outside supported rule scopes.
- Items that require permissions the app does not have and whose risk cannot be determined.

## Configuration Reference Protection

The app must protect against deleting folders that appear to be cache or old versions but are still used as runtime entry points.

This requirement comes from a real failure pattern:

- A tool configuration referenced an `npm-cache/_npx/.../exa-mcp-server` path.
- A tool configuration referenced an old VS Code extension path, `highagency.pencildev-0.6.51`.
- Cleaning those folders broke tool startup even though the folders looked like cache or old versions.

The scanner must inspect known configuration locations for path references before classifying a candidate as cleanable.

Known locations in the first version:

- User-level tool config files under the user profile.
- Editor extension metadata where available.
- Common MCP and agent config files where they are plain text and local.

If a candidate path is referenced by configuration:

- It is classified as Not Cleanable.
- The app explains that a tool still starts from this path.
- The app does not edit the configuration.
- The app may suggest that the user update the related tool manually.

## Process Usage Protection

If a candidate path is used by a running process:

- The item is not cleaned during the current run.
- The app shows "Close the related app and scan again."
- The app does not kill processes automatically.

The app may show the related application name when it can identify it without exposing sensitive path details.

## Deletion Policy

The app deletes selected items directly after confirmation. It does not move them to the Recycle Bin and does not maintain an internal quarantine in the first version.

Because deletion is direct:

- Only recommended items that can be cleaned in normal mode are selected by default.
- Optional and high-risk items require manual user selection.
- High-risk items require second confirmation.
- Not cleanable items cannot be selected.
- Each deletion must pass path allowlist checks immediately before execution.

## Administrator Mode

Administrator mode is optional and user-initiated.

V0.1 only exposes the administrator-mode entry point and explains the lightweight system cleanup capability. It does not execute elevated cleanup yet. V0.2 may add elevated execution for:

- Windows temporary directory.
- Windows Update download cache.
- System logs that are safe to remove.

Administrator mode must not:

- Run DISM component cleanup.
- Delete system restore points.
- Manually clean WinSxS.
- Modify protected system components.
- Run destructive system commands outside explicit supported rules.

Because V0.1 does not execute elevated cleanup, administrator permission is not requested during normal operation.

## Common Software Strategy

### WeChat

The app separates:

- Logs and clear cache: lower risk when confidently identified.
- Images and videos: high risk.
- Files and downloads: high risk.
- Chat databases: high risk.
- Unknown WeChat data: not cleanable.

### QQ

The app separates:

- Logs and clear cache: lower risk when confidently identified.
- Images, videos, group files, and downloads: high risk.
- Chat databases: high risk.
- Unknown QQ data: not cleanable.

### Lark, DingTalk, And WeCom

The app separates:

- Logs and update caches.
- Image and video caches.
- Downloaded files.
- Unknown internal databases.

Downloaded files and unknown databases are high risk or not cleanable unless rules can identify them precisely.

### Cloud Drives And Sync Tools

The app is conservative with cloud drives:

- It shows space usage.
- It explains possible sync risk.
- It classifies local-only or uncertain files as high risk or not cleanable.
- It does not assume that cloud files can be safely removed locally.

## Analytics And Privacy

Anonymous analytics are enabled by default. The first launch must clearly disclose this and provide a way to disable analytics.

Allowed analytics:

- App version.
- Windows major version.
- Scan duration bucket.
- Rule category hit, such as "system temporary files" or "WeChat image cache".
- Freed space bucket, such as `0-1GB`, `1-5GB`, or `5GB+`.
- Error category, such as permission denied or file in use.
- Whether analytics was disabled.

Forbidden analytics:

- Full paths.
- File names.
- User names.
- Configuration file contents.
- Chat records.
- Image, video, or document contents.
- Software account identifiers.
- Stable machine hardware identifiers.

Analytics failure must not interrupt scanning or cleaning.

## Packaging

The first version provides:

- Standard installer for ordinary users.
- Portable single EXE for technical users.

The installer should create Start Menu entries and support normal uninstall. Portable mode should keep settings local to the app folder when possible, except for OS-required temporary runtime data.

## Error Handling

Cleanup failures are shown in plain language:

- File is currently in use.
- Permission denied.
- Path no longer exists.
- Safety check failed.
- Administrator permission was cancelled, in versions that support elevated cleanup.
- Item is referenced by a tool configuration.

The app should avoid exposing long technical paths by default. Users can expand details when needed.

## Success Criteria

The first version succeeds if:

- A normal user can scan C drive and understand what is safe to clean.
- Recommended cleanup can be executed with explicit confirmation.
- Optional and high-risk items are never selected accidentally.
- Configuration-referenced paths are not cleaned.
- Process-used paths are skipped instead of forcibly terminated.
- The app can explain every skipped or failed item.
- The app does not upload sensitive local information.

## Implementation Planning Inputs

These technical decisions belong in the implementation plan rather than the product spec:

- Desktop framework selection.
- Exact rule file format.
- Exact Windows APIs for process-path detection.
- Exact signed analytics endpoint design.
- Installer technology.
- Visual mockup implementation details.
