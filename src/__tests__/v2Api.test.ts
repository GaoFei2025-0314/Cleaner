import { describe, expect, it, vi } from "vitest";
import {
  cancelOperation,
  getDefaultCleanerSettings,
  onCleanerOperationFinished,
  onCleanerOperationProgress,
  startDuplicateScan,
  startDuplicateScanPreview,
} from "../services/v2Api";

describe("v2Api browser preview", () => {
  it("returns V0.2 default settings", async () => {
    const settings = await getDefaultCleanerSettings();
    expect(settings.defaultScanDrives).toEqual(["C:"]);
    expect(settings.largeFileDefaultThresholdBytes).toBe(500 * 1024 * 1024);
    expect(settings.historyRetentionDays).toBe(30);
  });

  it("provides duplicate scan preview data without Tauri", async () => {
    const report = await startDuplicateScanPreview();
    expect(report.strictGroups.length).toBeGreaterThan(0);
    expect(report.strictGroups[0].files.length).toBeGreaterThan(1);
  });

  it("emits deterministic browser progress and stops after unsubscribe", async () => {
    vi.useFakeTimers();
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
    vi.useRealTimers();
  });

  it("emits cancelled finished payload in browser preview", async () => {
    vi.useFakeTimers();
    const finishedStatuses: string[] = [];
    const unsubscribe = await onCleanerOperationFinished((payload) => {
      finishedStatuses.push(payload.status);
    });

    const operation = await startDuplicateScan();
    await vi.advanceTimersByTimeAsync(40);
    const cancelled = await cancelOperation(operation.operationId);
    await vi.advanceTimersByTimeAsync(20);
    await unsubscribe();

    expect(cancelled).toBe(true);
    expect(finishedStatuses).toEqual(["cancelled"]);
    vi.useRealTimers();
  });
});
