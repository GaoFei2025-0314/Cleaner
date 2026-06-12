import { afterEach, describe, expect, it, vi } from "vitest";
import type { CleanerSettings, DuplicateCleanupRequest, MigrationRequest } from "../domain/v2";
import {
  mockCleanerSettings,
  mockDuplicateScanReport,
  mockLargeFileScanReport,
  mockMigrationResult,
} from "../services/v2MockData";

const defaultSettings: CleanerSettings = {
  protectedPaths: [],
  defaultScanDrives: ["C:"],
  duplicateDefaultStrategy: "cDriveFirstKeepNewest",
  largeFileDefaultThresholdBytes: 500 * 1024 * 1024,
  historyRetentionDays: 30,
  desktopShortcutEnabled: false,
  cDriveContextMenuEnabled: false,
  scheduledScanReminderEnabled: false,
};

const cleanupRequest: DuplicateCleanupRequest = {
  groups: [
    {
      groupId: "dup-strict-001",
      files: [{ entryId: "dup-strict-001-b", selected: true, protected: false }],
    },
  ],
  protectedOverrideConfirmed: false,
};

const migrationRequest: MigrationRequest = {
  selectedItemIds: ["large-c-001"],
  scanReport: mockLargeFileScanReport,
  targetFolder: "D:/CleanerPreview",
  originalFilePolicy: "moveOriginalToRecycleBin",
  protectedOverrideConfirmed: false,
};

afterEach(() => {
  vi.useRealTimers();
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  vi.unstubAllGlobals();
  vi.resetModules();
  vi.restoreAllMocks();
  vi.doUnmock("@tauri-apps/api/core");
  vi.doUnmock("@tauri-apps/api/event");
});

function enableTauriMode(): void {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
}

describe("v2Api browser preview", () => {
  it("returns V0.2 default settings", async () => {
    const { getDefaultCleanerSettings } = await import("../services/v2Api");

    const settings = await getDefaultCleanerSettings();

    expect(settings.protectedPaths).toEqual([]);
    expect(settings.defaultScanDrives).toEqual(["C:"]);
    expect(settings.largeFileDefaultThresholdBytes).toBe(500 * 1024 * 1024);
    expect(settings.historyRetentionDays).toBe(30);
  });

  it("provides duplicate scan preview data without Tauri", async () => {
    const { startDuplicateScanPreview } = await import("../services/v2Api");

    const report = await startDuplicateScanPreview();

    expect(report.strictGroups.length).toBeGreaterThan(0);
    expect(report.strictGroups[0].files.length).toBeGreaterThan(1);
  });

  it("keeps browser preview mocks free of full paths and raw fingerprints", () => {
    const serialized = JSON.stringify({
      mockCleanerSettings,
      mockDuplicateScanReport,
      mockLargeFileScanReport,
      mockMigrationResult,
    });

    expect(serialized).not.toMatch(/[A-Z]:\\/);
    expect(serialized).not.toMatch(/Users|Administrator/i);
    expect(serialized).not.toMatch(/[a-f0-9]{32,}/i);
    expect(serialized).not.toMatch(/mock-fingerprint|raw-hash/i);
  });

  it("emits deterministic browser progress and stops after unsubscribe", async () => {
    vi.useFakeTimers();
    const { onCleanerOperationProgress, startDuplicateScan } = await import("../services/v2Api");
    const progressPercents: number[] = [];
    const unsubscribe = await onCleanerOperationProgress((payload) => {
      progressPercents.push(payload.percent);
    });

    await startDuplicateScan();
    await vi.advanceTimersByTimeAsync(250);
    await unsubscribe();
    const countAfterUnsubscribe = progressPercents.length;

    await vi.advanceTimersByTimeAsync(1_000);
    expect(progressPercents).toContain(0);
    expect(progressPercents).toContain(100);
    expect(progressPercents.length).toBe(countAfterUnsubscribe);
  });

  it("stops active browser operations when all listeners unsubscribe", async () => {
    vi.useFakeTimers();
    const { onCleanerOperationFinished, onCleanerOperationProgress, startDuplicateScan } = await import(
      "../services/v2Api"
    );
    const progressPercents: number[] = [];
    const finishedStatuses: string[] = [];
    const unsubscribeProgress = await onCleanerOperationProgress((payload) => {
      progressPercents.push(payload.percent);
    });
    const unsubscribeFinished = await onCleanerOperationFinished((payload) => {
      finishedStatuses.push(payload.status);
    });

    await startDuplicateScan();
    await vi.advanceTimersByTimeAsync(75);
    await unsubscribeProgress();
    await unsubscribeFinished();
    const progressCountAfterUnsubscribe = progressPercents.length;

    await vi.advanceTimersByTimeAsync(1_000);
    expect(progressPercents).toEqual([0, 25]);
    expect(progressPercents).not.toContain(100);
    expect(progressPercents.length).toBe(progressCountAfterUnsubscribe);
    expect(finishedStatuses).toEqual([]);
  });

  it("emits cancelled finished payload with null result in browser preview", async () => {
    vi.useFakeTimers();
    const { cancelOperation, onCleanerOperationFinished, startDuplicateScan } = await import("../services/v2Api");
    const finishedPayloads: Array<{ status: string; result: unknown }> = [];
    const unsubscribe = await onCleanerOperationFinished((payload) => {
      finishedPayloads.push({ status: payload.status, result: payload.result });
    });

    const operation = await startDuplicateScan();
    await vi.advanceTimersByTimeAsync(40);
    const cancelled = await cancelOperation(operation.operationId);
    await vi.advanceTimersByTimeAsync(20);
    await unsubscribe();

    expect(cancelled).toBe(true);
    expect(finishedPayloads).toEqual([{ status: "cancelled", result: null }]);
  });
});

describe("v2Api Tauri contracts", () => {
  it("wraps the V0.2 command names and builds no-arg scan requests from Tauri settings", async () => {
    const invoke = vi.fn(async (command: string) => {
      if (command === "get_cleaner_settings" || command === "save_cleaner_settings") {
        return defaultSettings;
      }
      if (command === "list_operation_history") {
        return [];
      }
      if (command === "clear_operation_history") {
        return undefined;
      }
      if (command === "cancel_operation") {
        return true;
      }
      return { operationId: `op-${command}` };
    });
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
    enableTauriMode();
    const {
      cancelOperation,
      clearOperationHistory,
      getCleanerSettings,
      listOperationHistory,
      saveCleanerSettings,
      startDuplicateCleanup,
      startDuplicateScan,
      startLargeFileMigration,
      startLargeFileScan,
    } = await import("../services/v2Api");

    await getCleanerSettings();
    await saveCleanerSettings(defaultSettings);
    await listOperationHistory();
    await clearOperationHistory();
    await startDuplicateScan();
    await startDuplicateCleanup(cleanupRequest);
    await startLargeFileScan();
    await startLargeFileMigration(migrationRequest);
    await cancelOperation("op-1");

    expect(invoke.mock.calls.map(([command]) => command)).toEqual([
      "get_cleaner_settings",
      "save_cleaner_settings",
      "list_operation_history",
      "clear_operation_history",
      "get_cleaner_settings",
      "start_duplicate_scan",
      "start_duplicate_cleanup",
      "get_cleaner_settings",
      "start_large_file_scan",
      "start_large_file_migration",
      "cancel_operation",
    ]);
    expect(invoke).toHaveBeenCalledWith("start_duplicate_scan", {
      request: expect.objectContaining({ protectedPaths: [], selectedDrives: ["C:"] }),
    });
    expect(invoke).toHaveBeenCalledWith("start_large_file_scan", {
      request: expect.objectContaining({
        minSizeBytes: 500 * 1024 * 1024,
        protectedPaths: [],
        selectedDrives: ["C:"],
      }),
    });
  });

  it("awaits native event listener registration before resolving subscriptions", async () => {
    let resolveProgressListen: ((unsubscribe: () => void) => void) | undefined;
    let resolveFinishedListen: ((unsubscribe: () => void) => void) | undefined;
    const progressUnsubscribe = vi.fn();
    const finishedUnsubscribe = vi.fn();
    const listen = vi.fn((eventName: string) => {
      if (eventName === "cleaner-operation-progress") {
        return new Promise((resolve) => {
          resolveProgressListen = resolve;
        });
      }
      if (eventName === "cleaner-operation-finished") {
        return new Promise((resolve) => {
          resolveFinishedListen = resolve;
        });
      }
      throw new Error(`Unexpected event ${eventName}`);
    });
    vi.doMock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
    vi.doMock("@tauri-apps/api/event", () => ({ listen }));
    enableTauriMode();
    const { onCleanerOperationFinished, onCleanerOperationProgress } = await import("../services/v2Api");

    let progressResolved = false;
    const progressSubscription = onCleanerOperationProgress(() => undefined).then((unsubscribe) => {
      progressResolved = true;
      return unsubscribe;
    });
    await Promise.resolve();
    expect(progressResolved).toBe(false);
    expect(listen).toHaveBeenCalledWith("cleaner-operation-progress", expect.any(Function));
    resolveProgressListen?.(progressUnsubscribe);
    const unsubscribeProgress = await progressSubscription;
    expect(progressResolved).toBe(true);
    await unsubscribeProgress();
    expect(progressUnsubscribe).toHaveBeenCalledTimes(1);

    let finishedResolved = false;
    const finishedSubscription = onCleanerOperationFinished(() => undefined).then((unsubscribe) => {
      finishedResolved = true;
      return unsubscribe;
    });
    await Promise.resolve();
    expect(finishedResolved).toBe(false);
    expect(listen).toHaveBeenCalledWith("cleaner-operation-finished", expect.any(Function));
    resolveFinishedListen?.(finishedUnsubscribe);
    const unsubscribeFinished = await finishedSubscription;
    expect(finishedResolved).toBe(true);
    await unsubscribeFinished();
    expect(finishedUnsubscribe).toHaveBeenCalledTimes(1);
  });
});
