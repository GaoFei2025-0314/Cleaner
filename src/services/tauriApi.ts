import { invoke } from "@tauri-apps/api/core";
import type { CleanupResult, CleanupSelection, ScanReport } from "../domain/models";
import { mockReport } from "./mockReport";

export const isBrowserPreview = typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window);

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
