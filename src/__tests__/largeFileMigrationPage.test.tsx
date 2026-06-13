import "@testing-library/jest-dom/vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "../App";
import { LargeFileMigrationPage } from "../components/large-files/LargeFileMigrationPage";
import type {
  CleanerSettings,
  LargeFileScanReport,
  MigrationRequest,
  MigrationResult,
  OperationFinishedPayload,
  OperationProgressPayload,
} from "../domain/v2";

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

const report: LargeFileScanReport = {
  scannedFiles: 2400,
  skippedLocations: 3,
  totalBytes: 12 * 1024 * 1024 * 1024,
  cDriveBytes: 10 * 1024 * 1024 * 1024,
  otherDriveBytes: 2 * 1024 * 1024 * 1024,
  items: [
    largeItem("video-1", "video", "C:", "training-video.mp4", 3 * 1024 * 1024 * 1024, true),
    largeItem("archive-1", "archive", "C:", "backup.7z", 1 * 1024 * 1024 * 1024, true),
    largeItem("installer-1", "installer", "C:", "setup.exe", 900 * 1024 * 1024, true),
    largeItem("disk-1", "diskImage", "D:", "vm-image.iso", 2 * 1024 * 1024 * 1024, true),
    largeItem("document-1", "document", "C:", "audit.pdf", 700 * 1024 * 1024, true),
    largeItem("other-1", "other", "C:", "dataset.bin", 800 * 1024 * 1024, true),
    {
      ...largeItem("protected-1", "archive", "C:", "protected.zip", 600 * 1024 * 1024, false),
      visibleLocationHint: "C drive / Protected",
      protected: true,
      recommended: false,
    },
  ],
};

const migrationResult: MigrationResult = {
  copiedCount: 3,
  movedToRecycleBinCount: 2,
  skippedCount: 1,
  failedCount: 1,
  totalCopiedBytes: 5 * 1024 * 1024 * 1024,
  totalFreedBytes: 4 * 1024 * 1024 * 1024,
  cDriveFreedBytes: 4 * 1024 * 1024 * 1024,
  itemResults: [
    resultItem("video-1", "copiedAndFreed", 3 * 1024 * 1024 * 1024, 3 * 1024 * 1024 * 1024, "已迁移并释放。"),
    resultItem("disk-1", "copied", 2 * 1024 * 1024 * 1024, 0, "已复制，原文件保留。"),
    resultItem("protected-1", "skipped", 0, 0, "受保护位置跳过。"),
    resultItem("other-1", "failed", 0, 0, "复制失败。"),
  ],
};

const apiMock = vi.hoisted(() => ({
  progressListener: undefined as ((payload: OperationProgressPayload) => void) | undefined,
  finishedListener: undefined as ((payload: OperationFinishedPayload) => void) | undefined,
  unsubscribeProgress: vi.fn(async () => undefined),
  unsubscribeFinished: vi.fn(async () => undefined),
  getCleanerSettings: vi.fn(async () => settings),
  startLargeFileScan: vi.fn(async () => ({ operationId: "scan-op" })),
  startLargeFileMigration: vi.fn(async () => ({ operationId: "migration-op" })),
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

vi.mock("../services/v2Api", () => ({
  getCleanerSettings: apiMock.getCleanerSettings,
  startLargeFileScan: apiMock.startLargeFileScan,
  startLargeFileMigration: apiMock.startLargeFileMigration,
  cancelOperation: apiMock.cancelOperation,
  onCleanerOperationProgress: apiMock.onCleanerOperationProgress,
  onCleanerOperationFinished: apiMock.onCleanerOperationFinished,
}));

beforeEach(() => {
  settings.largeFileDefaultThresholdBytes = 500 * 1024 * 1024;
  apiMock.getCleanerSettings.mockImplementation(async () => settings);
  apiMock.startLargeFileScan.mockImplementation(async () => ({ operationId: "scan-op" }));
  apiMock.startLargeFileMigration.mockImplementation(async () => ({ operationId: "migration-op" }));
  apiMock.cancelOperation.mockImplementation(async () => true);
  apiMock.onCleanerOperationProgress.mockImplementation(async (listener) => {
    apiMock.progressListener = listener;
    return apiMock.unsubscribeProgress;
  });
  apiMock.onCleanerOperationFinished.mockImplementation(async (listener) => {
    apiMock.finishedListener = listener;
    return apiMock.unsubscribeFinished;
  });
  vi.spyOn(window, "confirm").mockReturnValue(false);
});

afterEach(() => {
  apiMock.progressListener = undefined;
  apiMock.finishedListener = undefined;
  vi.restoreAllMocks();
  vi.clearAllMocks();
});

describe("LargeFileMigrationPage", () => {
  it("shows initial migration settings", async () => {
    render(<LargeFileMigrationPage />);

    expect(await screen.findByRole("heading", { name: /大文件迁移/ })).toBeInTheDocument();
    expect(screen.getByLabelText(/C:/)).toBeChecked();
    expect(screen.getByRole("radio", { name: /100MB/ })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /500MB/ })).toBeChecked();
    expect(screen.getByRole("radio", { name: /1GB/ })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /自定义/ })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: /保留原文件/ })).toBeChecked();
    expect(screen.getByRole("button", { name: /开始扫描/ })).toBeEnabled();
  });

  it("uses custom MB threshold for large-file scan requests", async () => {
    render(<LargeFileMigrationPage />);

    await screen.findByRole("heading", { name: /大文件迁移/ });
    fireEvent.click(screen.getByRole("radio", { name: /自定义/ }));
    fireEvent.change(screen.getByLabelText(/自定义阈值/), { target: { value: "768" } });
    fireEvent.click(screen.getByRole("button", { name: /开始扫描/ }));

    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalledWith(expect.objectContaining({
      minSizeBytes: 768 * 1024 * 1024,
    })));
  });

  it("sends custom folders with large-file scan requests", async () => {
    render(<LargeFileMigrationPage />);

    await screen.findByRole("heading", { name: /大文件迁移/ });
    fireEvent.change(screen.getByLabelText(/文件夹路径/), { target: { value: "E:\\Media" } });
    fireEvent.click(screen.getByRole("button", { name: /添加文件夹/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始扫描/ }));

    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalledWith(expect.objectContaining({
      customFolders: ["E:\\Media"],
    })));
  });

  it("shows scan progress percent and found large-file bytes", async () => {
    render(<LargeFileMigrationPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalledWith(expect.objectContaining({
      minSizeBytes: 500 * 1024 * 1024,
      protectedPaths: ["C drive / Protected"],
      selectedDrives: ["C:"],
    })));

    act(() => {
      apiMock.progressListener?.(progress("scan-op", "largeFileScan", {
        stage: "正在识别大文件",
        percent: 42,
        currentLocationHint: "C drive / Downloads",
        scannedFiles: 1200,
        foundItems: 5,
        foundBytes: 3 * 1024 * 1024 * 1024,
      }));
    });

    expect(await screen.findByText("42%")).toBeInTheDocument();
    expect(screen.getByText("正在识别大文件")).toBeInTheDocument();
    expect(screen.getByText("C drive / Downloads")).toBeInTheDocument();
    expect(screen.getByText("已发现大文件").parentElement).toHaveTextContent("3.0 GB");
    expect(screen.getByText("发现项目").parentElement).toHaveTextContent("5");
    expect(screen.getByText("已扫描文件").parentElement).toHaveTextContent("1,200");
  });

  it("groups scan results by large file category", async () => {
    await renderResults();

    for (const name of ["视频", "压缩包", "安装包", "磁盘映像", "文档", "其他"]) {
      expect(screen.getByRole("heading", { name })).toBeInTheDocument();
    }
    expect(screen.getByLabelText(/protected\.zip.*受保护/u)).toBeDisabled();
    expect(screen.getByLabelText(/protected\.zip.*受保护/u)).not.toBeChecked();
  });

  it("blocks migration confirmation when target folder is empty", async () => {
    await renderResults();

    fireEvent.click(screen.getByRole("button", { name: /设置迁移目标/ }));
    const target = screen.getByLabelText(/目标文件夹/);
    fireEvent.change(target, { target: { value: "" } });

    expect(screen.getByRole("button", { name: /确认迁移内容/ })).toBeDisabled();
  });

  it("changes expected freed space when original policy moves files to recycle bin", async () => {
    await renderResults();

    fireEvent.click(screen.getByRole("button", { name: /设置迁移目标/ }));
    expect(screen.getByText("预计释放原位置空间").parentElement).toHaveTextContent("0 B");
    expect(screen.getByText(/选择移入回收站才会释放原位置空间/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("radio", { name: /迁移后将原文件移入回收站/ }));

    expect(screen.getByText("预计释放原位置空间").parentElement).toHaveTextContent("8.3 GB");
  });

  it("gates starting migration with a confirmation checkbox", async () => {
    await renderConfirm();

    expect(screen.getByRole("button", { name: /开始迁移/ })).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    expect(screen.getByRole("button", { name: /开始迁移/ })).toBeEnabled();
  });

  it("distinguishes migrated space from actual freed space in final results", async () => {
    await renderConfirm();
    fireEvent.click(screen.getByRole("checkbox", { name: /我已确认/ }));
    fireEvent.click(screen.getByRole("button", { name: /开始迁移/ }));

    await waitFor(() => expect(apiMock.startLargeFileMigration).toHaveBeenCalledWith(expect.objectContaining({
      originalFilePolicy: "moveOriginalToRecycleBin",
      targetFolder: "D:\\Cleaner_MigratedFiles",
    } satisfies Partial<MigrationRequest>)));

    act(() => {
      apiMock.finishedListener?.({
        operationId: "migration-op",
        module: "largeFileMigration",
        status: "completed",
        result: migrationResult,
        message: null,
      });
    });

    expect((await screen.findAllByText("迁移成功"))[0].parentElement).toHaveTextContent("5.0 GB");
    expect(screen.getByText("已释放原位置空间").parentElement).toHaveTextContent("4.0 GB");
    expect(screen.getAllByText("迁移成功但未释放空间")[0].parentElement).toHaveTextContent("1.0 GB");
    expect(screen.getAllByText("已跳过")[0].parentElement).toHaveTextContent("1");
    expect(screen.getAllByText("失败")[0].parentElement).toHaveTextContent("1");
  });

  it("applies matching scan finished events received before start resolves", async () => {
    const start = deferred<{ operationId: string }>();
    apiMock.startLargeFileScan.mockReturnValueOnce(start.promise);
    render(<LargeFileMigrationPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalled());

    act(() => {
      apiMock.finishedListener?.({
        operationId: "scan-op",
        module: "largeFileScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    await act(async () => {
      start.resolve({ operationId: "scan-op" });
      await start.promise;
    });

    expect(await screen.findByRole("heading", { name: /扫描结果/ })).toBeInTheDocument();
  });

  it("ignores mismatched scan finished events received before start resolves", async () => {
    const start = deferred<{ operationId: string }>();
    apiMock.startLargeFileScan.mockReturnValueOnce(start.promise);
    render(<LargeFileMigrationPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalled());

    act(() => {
      apiMock.finishedListener?.({
        operationId: "other-scan-op",
        module: "largeFileScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    await act(async () => {
      start.resolve({ operationId: "scan-op" });
      await start.promise;
    });

    expect(screen.queryByRole("heading", { name: /扫描结果/ })).not.toBeInTheDocument();
    expect(screen.getByText("正在启动扫描")).toBeInTheDocument();

    act(() => {
      apiMock.finishedListener?.({
        operationId: "scan-op",
        module: "largeFileScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    expect(await screen.findByRole("heading", { name: /扫描结果/ })).toBeInTheDocument();
  });

  it("cleans up listeners and cancels an active large-file operation on unmount", async () => {
    const { unmount } = render(<LargeFileMigrationPage />);

    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalled());

    unmount();

    await waitFor(() => expect(apiMock.unsubscribeProgress).toHaveBeenCalledTimes(1));
    expect(apiMock.unsubscribeFinished).toHaveBeenCalledTimes(1);
    expect(apiMock.cancelOperation).toHaveBeenCalledWith("scan-op");
  });

  it("reports blocking work for App while large-file scan or unfinished flow is active", async () => {
    render(<App />);

    fireEvent.click(await screen.findByRole("button", { name: "大文件迁移" }));
    fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
    await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    expect(window.confirm).toHaveBeenCalledWith("当前清理任务或选择尚未完成，确定要切换页面吗？");
    expect(screen.getByText("正在启动扫描")).toBeInTheDocument();

    act(() => {
      apiMock.finishedListener?.({
        operationId: "scan-op",
        module: "largeFileScan",
        status: "completed",
        result: report,
        message: null,
      });
    });

    await screen.findByRole("heading", { name: /扫描结果/ });
    fireEvent.click(screen.getByRole("button", { name: "设置" }));
    expect(window.confirm).toHaveBeenCalledTimes(2);
  });
});

async function renderResults() {
  render(<LargeFileMigrationPage />);
  fireEvent.click(await screen.findByRole("button", { name: /开始扫描/ }));
  await waitFor(() => expect(apiMock.startLargeFileScan).toHaveBeenCalled());
  act(() => {
    apiMock.finishedListener?.({
      operationId: "scan-op",
      module: "largeFileScan",
      status: "completed",
      result: report,
      message: null,
    });
  });
  await screen.findByRole("heading", { name: /扫描结果/ });
}

async function renderConfirm() {
  await renderResults();
  fireEvent.click(screen.getByRole("button", { name: /设置迁移目标/ }));
  fireEvent.click(screen.getByRole("radio", { name: /迁移后将原文件移入回收站/ }));
  fireEvent.click(screen.getByRole("button", { name: /确认迁移内容/ }));
  await screen.findByRole("heading", { name: /确认迁移/ });
}

function largeItem(
  itemId: string,
  category: LargeFileScanReport["items"][number]["category"],
  drive: string,
  displayName: string,
  sizeBytes: number,
  recommended: boolean,
): LargeFileScanReport["items"][number] {
  return {
    itemId,
    displayName,
    drive,
    visibleLocationHint: `${drive.slice(0, 1)} drive / Downloads`,
    sizeBytes,
    modifiedAt: "2026-05-01T00:00:00.000Z",
    category,
    selected: recommended,
    protected: false,
    recommended,
  };
}

function resultItem(
  itemId: string,
  status: MigrationResult["itemResults"][number]["status"],
  bytesCopied: number,
  bytesFreed: number,
  message: string,
): MigrationResult["itemResults"][number] {
  return {
    itemId,
    status,
    category: "video",
    bytesCopied,
    bytesFreed,
    message,
  };
}

function progress(
  operationId: string,
  module: OperationProgressPayload["module"],
  overrides: Partial<OperationProgressPayload>,
): OperationProgressPayload {
  return {
    operationId,
    module,
    stage: "准备中",
    percent: 0,
    currentLocationHint: "",
    currentFileType: null,
    scannedFiles: 0,
    foundGroups: 0,
    foundItems: 0,
    foundBytes: 0,
    processedItems: 0,
    successCount: 0,
    skippedCount: 0,
    failedCount: 0,
    ...overrides,
  };
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
