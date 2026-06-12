import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  CleanerSettings,
  DuplicateCleanupReport,
  DuplicateCleanupRequest,
  DuplicateScanReport,
  DuplicateScanRequest,
  HistoryEntry,
  LargeFileScanReport,
  LargeFileScanRequest,
  MigrationRequest,
  MigrationResult,
  OperationFinishedPayload,
  OperationModule,
  OperationProgressPayload,
  OperationStart,
} from "../domain/v2";
import { isBrowserPreview } from "./tauriApi";
import {
  cloneMock,
  mockCleanerSettings,
  mockDuplicateCleanupReport,
  mockDuplicateScanReport,
  mockHistory,
  mockLargeFileScanReport,
  mockMigrationResult,
} from "./v2MockData";

const PROGRESS_EVENT = "cleaner-operation-progress";
const FINISHED_EVENT = "cleaner-operation-finished";

type CleanerUnsubscribe = () => Promise<void>;
type ProgressListener = (payload: OperationProgressPayload) => void;
type FinishedListener = (payload: OperationFinishedPayload) => void;

interface BrowserOperation {
  operationId: string;
  module: OperationModule;
  timers: number[];
  finished: boolean;
}

const progressListeners = new Set<ProgressListener>();
const finishedListeners = new Set<FinishedListener>();
const browserOperations = new Map<string, BrowserOperation>();
let browserOperationCounter = 0;
let browserSettings = cloneMock(mockCleanerSettings);
let browserHistory = cloneMock(mockHistory);

export async function getDefaultCleanerSettings(): Promise<CleanerSettings> {
  return getCleanerSettings();
}

export async function getCleanerSettings(): Promise<CleanerSettings> {
  if (isBrowserPreview) {
    return cloneMock(browserSettings);
  }
  return invoke<CleanerSettings>("get_cleaner_settings");
}

export async function saveCleanerSettings(settings: CleanerSettings): Promise<CleanerSettings> {
  if (isBrowserPreview) {
    browserSettings = cloneMock(settings);
    return cloneMock(browserSettings);
  }
  return invoke<CleanerSettings>("save_cleaner_settings", { settings });
}

export async function listOperationHistory(): Promise<HistoryEntry[]> {
  if (isBrowserPreview) {
    return cloneMock(browserHistory);
  }
  return invoke<HistoryEntry[]>("list_operation_history");
}

export async function clearOperationHistory(): Promise<void> {
  if (isBrowserPreview) {
    browserHistory = [];
    return;
  }
  return invoke<void>("clear_operation_history");
}

export async function startDuplicateScanPreview(): Promise<DuplicateScanReport> {
  return cloneMock(mockDuplicateScanReport);
}

export async function startDuplicateScan(request = defaultDuplicateScanRequest()): Promise<OperationStart> {
  if (isBrowserPreview) {
    return startBrowserOperation("duplicateScan", cloneMock(mockDuplicateScanReport));
  }
  return invoke<OperationStart>("start_duplicate_scan", { request });
}

export async function startDuplicateCleanup(request: DuplicateCleanupRequest): Promise<OperationStart> {
  if (isBrowserPreview) {
    return startBrowserOperation("duplicateCleanup", cloneMock(mockDuplicateCleanupReport));
  }
  return invoke<OperationStart>("start_duplicate_cleanup", { request });
}

export async function startLargeFileScan(request = defaultLargeFileScanRequest()): Promise<OperationStart> {
  if (isBrowserPreview) {
    return startBrowserOperation("largeFileScan", cloneMock(mockLargeFileScanReport));
  }
  return invoke<OperationStart>("start_large_file_scan", { request });
}

export async function startLargeFileMigration(request: MigrationRequest): Promise<OperationStart> {
  if (isBrowserPreview) {
    return startBrowserOperation("largeFileMigration", cloneMock(mockMigrationResult));
  }
  return invoke<OperationStart>("start_large_file_migration", { request });
}

export async function cancelOperation(operationId: string): Promise<boolean> {
  if (isBrowserPreview) {
    const operation = browserOperations.get(operationId);
    if (!operation || operation.finished) {
      return false;
    }
    finishBrowserOperation(operation, "cancelled", {}, "浏览器预览模式已取消操作。");
    return true;
  }
  return invoke<boolean>("cancel_operation", { operationId });
}

export async function onCleanerOperationProgress(listener: ProgressListener): Promise<CleanerUnsubscribe> {
  if (isBrowserPreview) {
    progressListeners.add(listener);
    return async () => {
      progressListeners.delete(listener);
      stopBrowserOperationsWhenUnobserved();
    };
  }

  const unlistenPromise = listen<OperationProgressPayload>(PROGRESS_EVENT, (event) => {
    listener(event.payload);
  });
  return stableUnsubscribe(unlistenPromise);
}

export async function onCleanerOperationFinished(listener: FinishedListener): Promise<CleanerUnsubscribe> {
  if (isBrowserPreview) {
    finishedListeners.add(listener);
    return async () => {
      finishedListeners.delete(listener);
      stopBrowserOperationsWhenUnobserved();
    };
  }

  const unlistenPromise = listen<OperationFinishedPayload>(FINISHED_EVENT, (event) => {
    listener(event.payload);
  });
  return stableUnsubscribe(unlistenPromise);
}

function stableUnsubscribe(unlistenPromise: Promise<UnlistenFn>): CleanerUnsubscribe {
  let unsubscribed = false;
  return async () => {
    if (unsubscribed) {
      return;
    }
    unsubscribed = true;
    const unlisten = await unlistenPromise;
    unlisten();
  };
}

function startBrowserOperation<T>(module: OperationModule, result: T): OperationStart {
  const operationId = `browser-${module}-${++browserOperationCounter}`;
  const operation: BrowserOperation = {
    operationId,
    module,
    timers: [],
    finished: false,
  };
  browserOperations.set(operationId, operation);

  const progressSteps = [0, 25, 60, 85, 100];
  progressSteps.forEach((percent, index) => {
    const timer = window.setTimeout(() => {
      if (!operation.finished) {
        emitProgress(browserProgressPayload(operation, percent));
      }
    }, index * 50);
    operation.timers.push(timer);
  });

  const finishTimer = window.setTimeout(() => {
    finishBrowserOperation(operation, "completed", result, "浏览器预览模式模拟完成。");
  }, progressSteps.length * 50);
  operation.timers.push(finishTimer);

  return { operationId };
}

function finishBrowserOperation<T>(
  operation: BrowserOperation,
  status: OperationFinishedPayload<T>["status"],
  result: T,
  message: string | null,
): void {
  if (operation.finished) {
    return;
  }
  operation.finished = true;
  operation.timers.forEach((timer) => window.clearTimeout(timer));
  operation.timers = [];
  browserOperations.delete(operation.operationId);

  emitFinished({
    operationId: operation.operationId,
    module: operation.module,
    status,
    result,
    message,
  });
}

function stopBrowserOperationsWhenUnobserved(): void {
  if (progressListeners.size > 0 || finishedListeners.size > 0) {
    return;
  }
  for (const operation of browserOperations.values()) {
    operation.timers.forEach((timer) => window.clearTimeout(timer));
    operation.timers = [];
    browserOperations.delete(operation.operationId);
  }
}

function emitProgress(payload: OperationProgressPayload): void {
  progressListeners.forEach((listener) => listener(payload));
}

function emitFinished(payload: OperationFinishedPayload): void {
  finishedListeners.forEach((listener) => listener(payload));
}

function browserProgressPayload(operation: BrowserOperation, percent: number): OperationProgressPayload {
  return {
    operationId: operation.operationId,
    module: operation.module,
    stage: percent === 100 ? "完成预览" : "浏览器预览扫描中",
    percent,
    currentLocationHint: "C:\\...\\Preview",
    currentFileType: operation.module === "largeFileScan" ? "video" : "document",
    scannedFiles: Math.round(420 * (percent / 100)),
    foundGroups: Math.round(mockDuplicateScanReport.strictGroups.length * (percent / 100)),
    foundItems: Math.round(mockLargeFileScanReport.items.length * (percent / 100)),
    foundBytes: Math.round(mockLargeFileScanReport.totalBytes * (percent / 100)),
    processedItems: Math.round(3 * (percent / 100)),
    successCount: percent === 100 ? 1 : 0,
    skippedCount: 0,
    failedCount: 0,
  };
}

function defaultDuplicateScanRequest(): DuplicateScanRequest {
  return {
    selectedDrives: ["C:"],
    customFolders: [],
    fileTypes: ["image", "document", "audio", "video", "archive"],
    customExtensions: [],
    includeSuspected: true,
    minSizeBytes: 1,
    protectedPaths: cloneMock(browserSettings.protectedPaths),
  };
}

function defaultLargeFileScanRequest(): LargeFileScanRequest {
  return {
    selectedDrives: ["C:"],
    customFolders: [],
    minSizeBytes: browserSettings.largeFileDefaultThresholdBytes,
    protectedPaths: cloneMock(browserSettings.protectedPaths),
    skipSystemDirs: true,
    skipProgramDirs: true,
  };
}
