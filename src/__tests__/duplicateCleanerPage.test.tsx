import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CleanerSettings,
  DuplicateCleanupReport,
  DuplicateCleanupRequest,
  DuplicateScanReport,
  DuplicateScanRequest,
  OperationFinishedPayload,
  OperationProgressPayload,
} from "../domain/v2";
import { DuplicateCleanerPage } from "../components/duplicate/DuplicateCleanerPage";

const settings: CleanerSettings = {
  protectedPaths: ["C drive / Protected"],
  defaultScanDrives: ["C:"],
  duplicateDefaultStrategy: "cDriveFirstKeepNewest",
  largeFileDefaultThresholdBytes: 500 * 1024 * 1024,
  historyRetentionDays: 30,
  desktopShortcutEnabled: false,
  cDriveContextMenuEnabled: false,
  scheduledScanReminderEnabled: false,
};

const report: DuplicateScanReport = {
  strictGroups: [
    {
      groupId: "strict-a",
      strictDuplicate: true,
      totalBytes: 300,
      reclaimableBytes: 200,
      recommendedSelectionReason: "建议保留 C 盘副本。",
      files: [
        {
          entryId: "strict-a-c",
          displayName: "重复文件 1",
          drive: "C:",
          visibleLocationHint: "C 盘 · 文件夹",
          sizeBytes: 100,
          modifiedAt: "2026-05-03T00:00:00.000Z",
          hashFingerprintId: "hidden-strict-a",
          selected: false,
          protected: false,
          recommendedAction: "keep",
        },
        {
          entryId: "strict-a-d",
          displayName: "重复文件 2",
          drive: "D:",
          visibleLocationHint: "D 盘 · 文件夹",
          sizeBytes: 100,
          modifiedAt: "2026-05-02T00:00:00.000Z",
          hashFingerprintId: "hidden-strict-a",
          selected: true,
          protected: false,
          recommendedAction: "clean",
        },
        {
          entryId: "strict-a-protected",
          displayName: "重复文件 3",
          drive: "C:",
          visibleLocationHint: "C 盘 · 文件夹",
          sizeBytes: 100,
          modifiedAt: "2026-05-01T00:00:00.000Z",
          hashFingerprintId: "hidden-strict-a",
          selected: true,
          protected: true,
          recommendedAction: "clean",
        },
      ],
    },
  ],
  suspectedGroups: [
    {
      groupId: "suspected-a",
      strictDuplicate: false,
      totalBytes: 400,
      reclaimableBytes: 0,
      recommendedSelectionReason: "文件名相近，建议手动确认。",
      files: [
        {
          entryId: "suspected-a-1",
          displayName: "疑似重复 1",
          drive: "C:",
          visibleLocationHint: "C 盘 · 文件夹",
          sizeBytes: 200,
          modifiedAt: "2026-05-03T00:00:00.000Z",
          hashFingerprintId: "hidden-suspected-a",
          selected: false,
          protected: false,
          recommendedAction: "manualReview",
        },
        {
          entryId: "suspected-a-2",
          displayName: "疑似重复 2",
          drive: "E:",
          visibleLocationHint: "E 盘 · 文件夹",
          sizeBytes: 200,
          modifiedAt: "2026-05-02T00:00:00.000Z",
          hashFingerprintId: "hidden-suspected-b",
          selected: false,
          protected: false,
          recommendedAction: "manualReview",
        },
      ],
    },
  ],
  scannedFiles: 1200,
  skippedLocations: 2,
  totalReclaimableBytes: 200,
  cDriveReclaimableBytes: 0,
  otherDriveReclaimableBytes: 100,
};

const cleanupReport: DuplicateCleanupReport = {
  processedFiles: 1,
  successCount: 1,
  skippedCount: 2,
  failedCount: 3,
  freedBytes: 100,
  cDriveFreedBytes: 0,
  otherDriveFreedBytes: 100,
};

const apiMock = vi.hoisted(() => ({
  progressListener: undefined as ((payload: OperationProgressPayload) => void) | undefined,
  finishedListener: undefined as ((payload: OperationFinishedPayload) => void) | undefined,
  unsubscribeProgress: vi.fn(async () => undefined),
  unsubscribeFinished: vi.fn(async () => undefined),
  getCleanerSettings: vi.fn(async () => settings),
  startDuplicateScan: vi.fn(async () => ({ operationId: "scan-op" })),
  startDuplicateCleanup: vi.fn(async () => ({ operationId: "clean-op" })),
  cancelOperation: vi.fn(async () => true),
  onCleanerOperationProgress: vi.fn(async (listener) => {
    apiMock.progressListener = listener;
    return apiMock.unsubscribeProgress;
  }),
  onCleanerOperationFinished: vi.fn(async (listener) => {
    apiMock.finishedListener = listener;
    return apiMock.unsubscribeFinished;
  }),
}));

vi.mock("../services/v2Api", () => {
  return {
    getCleanerSettings: apiMock.getCleanerSettings,
    startDuplicateScan: apiMock.startDuplicateScan,
    startDuplicateCleanup: apiMock.startDuplicateCleanup,
    cancelOperation: apiMock.cancelOperation,
    onCleanerOperationProgress: apiMock.onCleanerOperationProgress,
    onCleanerOperationFinished: apiMock.onCleanerOperationFinished,
  };
});

beforeEach(() => {
  settings.defaultScanDrives = ["C:"];
  settings.duplicateDefaultStrategy = "cDriveFirstKeepNewest";
  apiMock.getCleanerSettings.mockImplementation(async () => settings);
  apiMock.startDuplicateScan.mockImplementation(async () => ({ operationId: "scan-op" }));
  apiMock.startDuplicateCleanup.mockImplementation(async () => ({ operationId: "clean-op" }));
  apiMock.cancelOperation.mockImplementation(async () => true);
  apiMock.onCleanerOperationProgress.mockImplementation(async (listener) => {
    apiMock.progressListener = listener;
    return apiMock.unsubscribeProgress;
  });
  apiMock.onCleanerOperationFinished.mockImplementation(async (listener) => {
    apiMock.finishedListener = listener;
    return apiMock.unsubscribeFinished;
  });
});

afterEach(() => {
  apiMock.progressListener = undefined;
  apiMock.finishedListener = undefined;
  vi.clearAllMocks();
});

describe("DuplicateCleanerPage", () => {
  it("waits for settings before enabling duplicate scans", async () => {
    const settingsLoad = deferred<CleanerSettings>();
    apiMock.getCleanerSettings.mockReturnValueOnce(settingsLoad.promise);
    render(<DuplicateCleanerPage />);

    const startButton = screen.getByRole("button", { name: /正在加载设置/ });
    expect(startButton).toBeDisabled();

    fireEvent.click(startButton);
    expect(apiMock.startDuplicateScan).not.toHaveBeenCalled();

    await act(async () => {
      settingsLoad.resolve({
        ...settings,
        protectedPaths: ["C drive / Settings Protected"],
      });
      await settingsLoad.promise;
    });

    const enabledStartButton = await screen.findByRole("button", { name: /开始扫描/ });
    expect(enabledStartButton).toBeEnabled();

    fireEvent.click(enabledStartButton);

    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalledWith(expect.objectContaining({
      protectedPaths: ["C drive / Settings Protected"],
    } satisfies Partial<DuplicateScanRequest>)));
  });

  it("shows default duplicate scan settings without large-file threshold copy", async () => {
    render(<DuplicateCleanerPage />);

    expect(await screen.findByRole("heading", { name: /重复文件清理/ })).toBeInTheDocument();
    expect(screen.getByLabelText(/图片/)).toBeChecked();
    expect(screen.getByLabelText(/文档/)).toBeChecked();
    expect(screen.getByLabelText(/压缩包/)).toBeChecked();
    expect(screen.getByLabelText(/音频/)).not.toBeChecked();
    expect(screen.getByLabelText(/视频/)).not.toBeChecked();
    expect(screen.queryByText(/500/)).not.toBeInTheDocument();
  });

  it("shows scanning progress fields from operation events", async () => {
    render(<DuplicateCleanerPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalledWith(expect.objectContaining({
      fileTypes: ["image", "document", "archive"],
      includeSuspected: false,
      minSizeBytes: 1,
      protectedPaths: ["C drive / Protected"],
      selectedDrives: ["C:"],
    } satisfies Partial<DuplicateScanRequest>)));

    act(() => {
      apiMock.progressListener?.({
        operationId: "scan-op",
        module: "duplicateScan",
        stage: "正在比对指纹",
        percent: 44,
        currentLocationHint: "C 盘 · 文件夹",
        currentFileType: "document",
        scannedFiles: 321,
        foundGroups: 4,
        foundItems: 9,
        foundBytes: 100,
        processedItems: 0,
        successCount: 0,
        skippedCount: 0,
        failedCount: 0,
      });
    });

    expect(await screen.findByText("44%")).toBeInTheDocument();
    expect(screen.getByText("正在比对指纹")).toBeInTheDocument();
    expect(screen.getByText("C 盘 · 文件夹")).toBeInTheDocument();
    expect(screen.getByText("321")).toBeInTheDocument();
    expect(screen.getByText("发现重复组").parentElement).toHaveTextContent("4");
    expect(screen.getByText("发现文件").parentElement).toHaveTextContent("9");
  });

  it("cleans up active duplicate operation listeners and cancels when unmounted", async () => {
    apiMock.unsubscribeProgress.mockClear();
    apiMock.unsubscribeFinished.mockClear();
    apiMock.cancelOperation.mockClear();
    const { unmount } = render(<DuplicateCleanerPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalled());

    unmount();

    await waitFor(() => expect(apiMock.unsubscribeProgress).toHaveBeenCalledTimes(1));
    expect(apiMock.unsubscribeFinished).toHaveBeenCalledTimes(1);
    expect(apiMock.cancelOperation).toHaveBeenCalledWith("scan-op");
  });

  it("reports duplicate blocking work while scanning and after unfinished results", async () => {
    const onBlockingWorkChange = vi.fn();
    render(<DuplicateCleanerPage onBlockingWorkChange={onBlockingWorkChange} />);

    await waitFor(() => expect(onBlockingWorkChange).toHaveBeenLastCalledWith(false));

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(onBlockingWorkChange).toHaveBeenLastCalledWith(true));

    act(() => {
      apiMock.finishedListener?.({
        operationId: "scan-op",
        module: "duplicateScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    await screen.findByRole("heading", { name: /扫描结果/ });
    expect(onBlockingWorkChange).toHaveBeenLastCalledWith(true);

    fireEvent.click(screen.getByRole("button", { name: /返回设置/ }));
    await waitFor(() => expect(onBlockingWorkChange).toHaveBeenLastCalledWith(false));
  });

  it("cleans up partial listeners and restores settings when duplicate scan subscription fails", async () => {
    apiMock.onCleanerOperationFinished.mockRejectedValueOnce(new Error("listener failed"));
    render(<DuplicateCleanerPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));

    expect(await screen.findByText(/重复文件扫描启动失败/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /开始扫描/ })).toBeInTheDocument();
    expect(apiMock.unsubscribeProgress).toHaveBeenCalledTimes(1);
    expect(apiMock.startDuplicateScan).not.toHaveBeenCalled();
  });

  it("applies matching scan finished events received before start resolves", async () => {
    const scanStart = deferred<{ operationId: string }>();
    apiMock.startDuplicateScan.mockReturnValueOnce(scanStart.promise);
    render(<DuplicateCleanerPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalled());
    await waitFor(() => expect(apiMock.finishedListener).toBeDefined());

    act(() => {
      apiMock.finishedListener?.({
        operationId: "scan-op",
        module: "duplicateScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    await act(async () => {
      scanStart.resolve({ operationId: "scan-op" });
      await scanStart.promise;
    });

    expect(await screen.findByRole("heading", { name: /扫描结果/ })).toBeInTheDocument();
  });

  it("ignores mismatched scan events received before start resolves", async () => {
    const scanStart = deferred<{ operationId: string }>();
    apiMock.startDuplicateScan.mockReturnValueOnce(scanStart.promise);
    render(<DuplicateCleanerPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalled());
    await waitFor(() => expect(apiMock.finishedListener).toBeDefined());

    act(() => {
      apiMock.progressListener?.({
        operationId: "early-scan-op",
        module: "duplicateScan",
        stage: "错误的提前进度",
        percent: 99,
        currentLocationHint: "D drive / Early",
        currentFileType: "document",
        scannedFiles: 999,
        foundGroups: 9,
        foundItems: 9,
        foundBytes: 9,
        processedItems: 0,
        successCount: 0,
        skippedCount: 0,
        failedCount: 0,
      });
      apiMock.finishedListener?.({
        operationId: "early-scan-op",
        module: "duplicateScan",
        status: "failed",
        result: null,
        message: "提前失败事件",
      });
    });

    expect(screen.queryByText("错误的提前进度")).not.toBeInTheDocument();
    expect(screen.queryByText("提前失败事件")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /扫描结果/ })).not.toBeInTheDocument();

    await act(async () => {
      scanStart.resolve({ operationId: "scan-op" });
      await scanStart.promise;
    });

    act(() => {
      apiMock.finishedListener?.({
        operationId: "scan-op",
        module: "duplicateScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    expect(await screen.findByRole("heading", { name: /扫描结果/ })).toBeInTheDocument();
  });

  it("keeps C drive selected for duplicate scans even when settings default drives differ", async () => {
    settings.defaultScanDrives = ["D:"];
    render(<DuplicateCleanerPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));

    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalledWith(expect.objectContaining({
      selectedDrives: ["C:"],
    } satisfies Partial<DuplicateScanRequest>)));
  });

  it("sends custom folders with duplicate scan requests", async () => {
    render(<DuplicateCleanerPage />);

    await screen.findByRole("heading", { name: /重复文件清理/ });
    fireEvent.change(screen.getByLabelText(/文件夹路径/), { target: { value: "D:\\Downloads" } });
    fireEvent.click(screen.getByRole("button", { name: /添加文件夹/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始扫描/ }));

    await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalledWith(expect.objectContaining({
      customFolders: ["D:\\Downloads"],
    } satisfies Partial<DuplicateScanRequest>)));
  });

  it("separates strict and suspected duplicate results and does not show raw fingerprints", async () => {
    await renderResults();

    expect(screen.getByRole("heading", { name: /严格重复/ })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /疑似重复/ })).toBeInTheDocument();
    expect(screen.getByText("重复文件 1")).toBeInTheDocument();
    expect(screen.getByText("重复文件 2")).toBeInTheDocument();
    expect(screen.getByText("重复文件 3")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /展开疑似重复/ }));
    expect(screen.getByText("疑似重复 1")).toBeInTheDocument();
    expect(screen.getByText(/疑似重复仅供查看/)).toBeInTheDocument();
    expect(screen.queryByText(/hidden-strict/)).not.toBeInTheDocument();
  });

  it("C drive first selects C drive duplicates when a non-C copy can be retained", async () => {
    await renderResults();

    fireEvent.click(screen.getByRole("button", { name: /C 盘优先/ }));

    const strictGroup = screen.getByTestId("duplicate-group-strict-a");
    expect(within(strictGroup).getByLabelText(/重复文件 1/)).toBeChecked();
    expect(within(strictGroup).getByLabelText(/重复文件 2/)).not.toBeChecked();
    expect(within(strictGroup).getByLabelText(/重复文件 3.*受保护/u)).not.toBeChecked();
    expect(
      within(strictGroup)
        .getAllByLabelText(/重复文件/)
        .filter((checkbox) => (checkbox as HTMLInputElement).checked).length,
    ).toBe(1);
    expect(screen.getByText("C 盘已选").parentElement).toHaveTextContent("100 B");

    const suspectedGroup = screen.getByTestId("duplicate-group-suspected-a");
    expect(
      within(suspectedGroup)
        .getAllByRole("checkbox")
        .some((checkbox) => (checkbox as HTMLInputElement).checked),
    ).toBe(false);
  });

  it("requires confirmation checkbox before cleanup is enabled", async () => {
    await renderResults();

    fireEvent.click(screen.getByRole("button", { name: /下一步/ }));

    expect(screen.getByText(/文件将移入回收站/)).toBeInTheDocument();
    expect(screen.getByText(/每个重复组至少保留 1 份/)).toBeInTheDocument();
    expect(screen.getByText(/保护路径不会被自动清理/)).toBeInTheDocument();
    expect(screen.getByText(/回收站失败时不会永久删除/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /开始清理/ })).toBeDisabled();

    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    expect(screen.getByRole("button", { name: /开始清理/ })).toBeEnabled();
  });

  it("does not auto-select protected files", async () => {
    await renderResults();

    const protectedFile = screen.getByLabelText(/重复文件 3.*受保护/u);
    expect(protectedFile).not.toBeChecked();
    expect(protectedFile).toBeDisabled();
    expect(screen.getByText("受保护")).toBeInTheDocument();
  });

  it("shows cleaning progress and finished moved, skipped, and failed counts", async () => {
    await renderResults();
    fireEvent.click(screen.getByRole("button", { name: /下一步/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始清理/ }));

    await waitFor(() => expect(apiMock.startDuplicateCleanup).toHaveBeenCalledWith(expect.objectContaining({
      protectedOverrideConfirmed: false,
    } satisfies Partial<DuplicateCleanupRequest>)));

    act(() => {
      apiMock.progressListener?.({
        operationId: "clean-op",
        module: "duplicateCleanup",
        stage: "正在移入回收站",
        percent: 67,
        currentLocationHint: "D 盘 · 文件夹",
        currentFileType: "document",
        scannedFiles: 0,
        foundGroups: 0,
        foundItems: 0,
        foundBytes: 0,
        processedItems: 3,
        successCount: 1,
        skippedCount: 2,
        failedCount: 3,
      });
    });

    expect(await screen.findByText("67%")).toBeInTheDocument();
    expect(screen.getByText("D 盘 · 文件夹")).toBeInTheDocument();
    expect(screen.getByText(/已移入回收站：1/)).toBeInTheDocument();
    expect(screen.getByText(/已跳过：2/)).toBeInTheDocument();
    expect(screen.getByText(/失败：3/)).toBeInTheDocument();

    act(() => {
      apiMock.finishedListener?.({
        operationId: "clean-op",
        module: "duplicateCleanup",
        status: "completed",
        result: cleanupReport,
        message: null,
      });
    });

    expect(await screen.findByText(/已移入回收站：1/)).toBeInTheDocument();
    expect(screen.getByText(/已跳过：2/)).toBeInTheDocument();
    expect(screen.getByText(/失败：3/)).toBeInTheDocument();
  });

  it("sends selected and retained files for groups included in cleanup", async () => {
    await renderResults();
    fireEvent.click(screen.getByRole("button", { name: /下一步/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始清理/ }));

    await waitFor(() => expect(apiMock.startDuplicateCleanup).toHaveBeenCalled());
    const cleanupCalls = apiMock.startDuplicateCleanup.mock.calls as unknown as [[DuplicateCleanupRequest]];
    const request = cleanupCalls[0][0];

    expect(request.groups).toEqual([
      {
        groupId: "strict-a",
        files: [
          { entryId: "strict-a-c", selected: true, protected: false },
          { entryId: "strict-a-d", selected: false, protected: false },
          { entryId: "strict-a-protected", selected: false, protected: true },
        ],
      },
    ]);
  });

  it("keeps suspected duplicates read-only and out of cleanup requests", async () => {
    await renderResults();
    fireEvent.click(screen.getByRole("button", { name: /展开疑似重复/ }));
    const suspectedGroup = screen.getByTestId("duplicate-group-suspected-a");

    within(suspectedGroup)
      .getAllByRole("checkbox")
      .forEach((checkbox) => expect(checkbox).toBeDisabled());

    fireEvent.click(screen.getByRole("button", { name: /下一步/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始清理/ }));

    await waitFor(() => expect(apiMock.startDuplicateCleanup).toHaveBeenCalled());
    const cleanupCalls = apiMock.startDuplicateCleanup.mock.calls as unknown as [[DuplicateCleanupRequest]];
    expect(cleanupCalls[0][0].groups.map((group) => group.groupId)).toEqual(["strict-a"]);
  });

  it("applies matching cleanup finished events received before start resolves", async () => {
    const cleanupStart = deferred<{ operationId: string }>();
    apiMock.startDuplicateCleanup.mockReturnValueOnce(cleanupStart.promise);
    await renderResults();
    fireEvent.click(screen.getByRole("button", { name: /下一步/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始清理/ }));
    await waitFor(() => expect(apiMock.startDuplicateCleanup).toHaveBeenCalled());

    act(() => {
      apiMock.finishedListener?.({
        operationId: "clean-op",
        module: "duplicateCleanup",
        status: "completed",
        result: cleanupReport,
        message: null,
      });
    });

    await act(async () => {
      cleanupStart.resolve({ operationId: "clean-op" });
      await cleanupStart.promise;
    });

    expect(await screen.findByRole("heading", { name: /清理完成/ })).toBeInTheDocument();
  });

  it("ignores mismatched cleanup events received before start resolves", async () => {
    const cleanupStart = deferred<{ operationId: string }>();
    apiMock.startDuplicateCleanup.mockReturnValueOnce(cleanupStart.promise);
    await renderResults();
    fireEvent.click(screen.getByRole("button", { name: /下一步/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始清理/ }));
    await waitFor(() => expect(apiMock.startDuplicateCleanup).toHaveBeenCalled());

    act(() => {
      apiMock.progressListener?.({
        operationId: "early-clean-op",
        module: "duplicateCleanup",
        stage: "错误的提前清理进度",
        percent: 88,
        currentLocationHint: "D drive / Early",
        currentFileType: "document",
        scannedFiles: 0,
        foundGroups: 0,
        foundItems: 0,
        foundBytes: 0,
        processedItems: 88,
        successCount: 0,
        skippedCount: 0,
        failedCount: 1,
      });
      apiMock.finishedListener?.({
        operationId: "early-clean-op",
        module: "duplicateCleanup",
        status: "failed",
        result: null,
        message: "提前清理失败事件",
      });
    });

    expect(screen.queryByText("错误的提前清理进度")).not.toBeInTheDocument();
    expect(screen.queryByText("提前清理失败事件")).not.toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: /清理完成/ })).not.toBeInTheDocument();

    await act(async () => {
      cleanupStart.resolve({ operationId: "clean-op" });
      await cleanupStart.promise;
    });

    act(() => {
      apiMock.finishedListener?.({
        operationId: "clean-op",
        module: "duplicateCleanup",
        status: "completed",
        result: cleanupReport,
        message: null,
      });
    });

    expect(await screen.findByRole("heading", { name: /清理完成/ })).toBeInTheDocument();
  });
});

async function renderResults() {
  render(<DuplicateCleanerPage />);
  fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
  await waitFor(() => expect(apiMock.startDuplicateScan).toHaveBeenCalled());
  act(() => {
    apiMock.finishedListener?.({
      operationId: "scan-op",
      module: "duplicateScan",
      status: "completed",
      result: report,
      message: null,
    });
  });
  await screen.findByRole("heading", { name: /扫描结果/ });
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}
