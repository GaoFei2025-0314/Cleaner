import { CheckCircle2, ShieldCheck, Truck, XCircle } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CleanerSettings,
  LargeFileCategory,
  LargeFileItem,
  LargeFileScanReport,
  LargeFileScanRequest,
  MigrationRequest,
  MigrationResult,
  OperationFinishedPayload,
  OperationProgressPayload,
  OriginalFilePolicy,
} from "../../domain/v2";
import {
  cancelOperation,
  getCleanerSettings,
  onCleanerOperationFinished,
  onCleanerOperationProgress,
  startLargeFileMigration,
  startLargeFileScan,
} from "../../services/v2Api";

export type LargeFileStep = "settings" | "scanning" | "results" | "migrationSettings" | "confirm" | "migrating" | "finished";

type CleanerUnsubscribe = () => Promise<void>;
type LargeFileMigrationPageProps = {
  onBlockingWorkChange?: (blocking: boolean) => void;
};

const thresholdOptions = [
  { label: "500MB", value: 500 * 1024 * 1024 },
  { label: "1GB", value: 1024 * 1024 * 1024 },
  { label: "2GB", value: 2 * 1024 * 1024 * 1024 },
];

const categoryLabels: Record<LargeFileCategory, string> = {
  video: "视频",
  archive: "压缩包",
  installer: "安装包",
  diskImage: "磁盘映像",
  document: "文档",
  other: "其他",
};

const categoryOrder: LargeFileCategory[] = ["video", "archive", "installer", "diskImage", "document", "other"];

const emptyProgress: OperationProgressPayload = {
  operationId: "",
  module: "largeFileScan",
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
};

export function LargeFileMigrationPage({ onBlockingWorkChange }: LargeFileMigrationPageProps = {}) {
  const [step, setStep] = useState<LargeFileStep>("settings");
  const [settings, setSettings] = useState<CleanerSettings | null>(null);
  const [selectedDrives, setSelectedDrives] = useState(["C:"]);
  const [minSizeBytes, setMinSizeBytes] = useState(500 * 1024 * 1024);
  const [originalFilePolicy, setOriginalFilePolicy] = useState<OriginalFilePolicy>("keepOriginal");
  const [targetFolder, setTargetFolder] = useState("D:\\Cleaner_MigratedFiles");
  const [report, setReport] = useState<LargeFileScanReport | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [confirmed, setConfirmed] = useState(false);
  const [progress, setProgress] = useState<OperationProgressPayload>(emptyProgress);
  const [migrationResult, setMigrationResult] = useState<MigrationResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const operationIdRef = useRef<string | null>(null);
  const mountedRef = useRef(true);
  const unsubscribeProgressRef = useRef<CleanerUnsubscribe | null>(null);
  const unsubscribeFinishedRef = useRef<CleanerUnsubscribe | null>(null);

  async function cleanupActiveOperation(cancelBackend: boolean) {
    const operationId = operationIdRef.current;
    const unsubscribeProgress = unsubscribeProgressRef.current;
    const unsubscribeFinished = unsubscribeFinishedRef.current;
    operationIdRef.current = null;
    unsubscribeProgressRef.current = null;
    unsubscribeFinishedRef.current = null;

    await Promise.allSettled([
      unsubscribeProgress?.(),
      unsubscribeFinished?.(),
      cancelBackend && operationId ? cancelOperation(operationId) : Promise.resolve(),
    ]);
  }

  function safelySetStep(nextStep: LargeFileStep) {
    if (mountedRef.current) setStep(nextStep);
  }

  function safelySetError(message: string) {
    if (mountedRef.current) setError(message);
  }

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      void cleanupActiveOperation(true);
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    void getCleanerSettings()
      .then((nextSettings) => {
        if (disposed) return;
        setSettings(nextSettings);
        setMinSizeBytes(nextSettings.largeFileDefaultThresholdBytes);
      })
      .catch(() => {
        if (disposed) return;
        setError("设置加载失败，请稍后重试。");
      });
    return () => {
      disposed = true;
    };
  }, []);

  const selectedItems = useMemo(() => {
    if (!report) return [];
    return report.items.filter((item) => selectedIds.has(item.itemId) && !item.protected);
  }, [report, selectedIds]);

  const selectedBytes = selectedItems.reduce((sum, item) => sum + item.sizeBytes, 0);
  const selectedCount = selectedItems.length;
  const selectedCDriveBytes = selectedItems
    .filter((item) => item.drive.toUpperCase().startsWith("C"))
    .reduce((sum, item) => sum + item.sizeBytes, 0);
  const expectedFreedBytes = originalFilePolicy === "moveOriginalToRecycleBin" ? selectedCDriveBytes : 0;

  useEffect(() => {
    const hasUnfinishedFlow = report !== null && ["results", "migrationSettings", "confirm"].includes(step);
    onBlockingWorkChange?.(step === "scanning" || step === "migrating" || hasUnfinishedFlow);
  }, [onBlockingWorkChange, report, step]);

  useEffect(() => {
    return () => {
      onBlockingWorkChange?.(false);
    };
  }, [onBlockingWorkChange]);

  function updateDrive(drive: string, checked: boolean) {
    const nextDrives = checked ? [...selectedDrives, drive] : selectedDrives.filter((item) => item !== drive);
    const uniqueDrives = nextDrives.length ? Array.from(new Set(nextDrives)) : ["C:"];
    setSelectedDrives(uniqueDrives);
    setTargetFolder(suggestTargetFolder(uniqueDrives));
  }

  async function beginScan() {
    if (!settings) {
      setError("设置仍在加载中，请稍后再开始扫描。");
      return;
    }

    const request: LargeFileScanRequest = {
      selectedDrives,
      customFolders: [],
      minSizeBytes,
      protectedPaths: settings.protectedPaths,
      skipSystemDirs: true,
      skipProgramDirs: true,
    };

    operationIdRef.current = null;
    setError(null);
    setProgress({ ...emptyProgress, module: "largeFileScan", stage: "正在启动扫描" });
    setStep("scanning");

    try {
      unsubscribeProgressRef.current = await onCleanerOperationProgress((payload) => {
        if (!mountedRef.current) return;
        if (payload.module !== "largeFileScan" || !operationIdRef.current || payload.operationId !== operationIdRef.current) return;
        setProgress(payload);
      });
      unsubscribeFinishedRef.current = await onCleanerOperationFinished((payload) => {
        if (!mountedRef.current) return;
        if (payload.module !== "largeFileScan" || !operationIdRef.current || payload.operationId !== operationIdRef.current) return;
        void cleanupActiveOperation(false);
        handleScanFinished(payload as OperationFinishedPayload<LargeFileScanReport>);
      });
      const operation = await startLargeFileScan(request);
      if (!mountedRef.current) {
        await cancelOperation(operation.operationId);
        return;
      }
      operationIdRef.current = operation.operationId;
    } catch {
      await cleanupActiveOperation(false);
      safelySetError("大文件扫描启动失败，本次未迁移任何文件。");
      safelySetStep("settings");
    }
  }

  function handleScanFinished(payload: OperationFinishedPayload<LargeFileScanReport>) {
    operationIdRef.current = null;
    if (payload.status === "completed" && payload.result) {
      setReport(payload.result);
      setSelectedIds(buildDefaultSelection(payload.result));
      setConfirmed(false);
      setMigrationResult(null);
      setStep("results");
      return;
    }
    setError(payload.status === "cancelled" ? "扫描已取消。" : payload.message || "扫描失败，请稍后重试。");
    setStep("settings");
  }

  async function beginMigration() {
    if (!report || targetFolder.trim() === "") return;
    operationIdRef.current = null;
    setError(null);
    setProgress({ ...emptyProgress, module: "largeFileMigration", stage: "正在启动迁移" });
    setStep("migrating");

    try {
      unsubscribeProgressRef.current = await onCleanerOperationProgress((payload) => {
        if (!mountedRef.current) return;
        if (payload.module !== "largeFileMigration" || !operationIdRef.current || payload.operationId !== operationIdRef.current) return;
        setProgress(payload);
      });
      unsubscribeFinishedRef.current = await onCleanerOperationFinished((payload) => {
        if (!mountedRef.current) return;
        if (payload.module !== "largeFileMigration" || !operationIdRef.current || payload.operationId !== operationIdRef.current) return;
        void cleanupActiveOperation(false);
        handleMigrationFinished(payload as OperationFinishedPayload<MigrationResult>);
      });
      const operation = await startLargeFileMigration(buildMigrationRequest(report, selectedIds, targetFolder, originalFilePolicy));
      if (!mountedRef.current) {
        await cancelOperation(operation.operationId);
        return;
      }
      operationIdRef.current = operation.operationId;
    } catch {
      await cleanupActiveOperation(false);
      safelySetError("迁移启动失败，本次没有移动任何文件。");
      safelySetStep("confirm");
    }
  }

  function handleMigrationFinished(payload: OperationFinishedPayload<MigrationResult>) {
    operationIdRef.current = null;
    if (payload.status === "completed" && payload.result) {
      setMigrationResult(payload.result);
      setStep("finished");
      return;
    }
    setError(payload.status === "cancelled" ? "迁移已取消。" : payload.message || "迁移失败，请稍后重试。");
    setStep("confirm");
  }

  async function cancelCurrentOperation() {
    if (!operationIdRef.current) return;
    await cancelOperation(operationIdRef.current);
  }

  function toggleItem(item: LargeFileItem, checked: boolean) {
    if (item.protected) return;
    setSelectedIds((current) => {
      const next = new Set(current);
      if (checked) next.add(item.itemId);
      else next.delete(item.itemId);
      return next;
    });
  }

  function resetFlow() {
    setReport(null);
    setSelectedIds(new Set());
    setConfirmed(false);
    setMigrationResult(null);
    setOriginalFilePolicy("keepOriginal");
    setStep("settings");
  }

  return (
    <div className="tool-page large-file-page">
      <header className="tool-header duplicate-header">
        <div>
          <p className="eyebrow">Large File Migration</p>
          <h2>大文件迁移</h2>
        </div>
        {error && <p className="duplicate-error">{error}</p>}
      </header>

      {step === "settings" && (
        <section className="settings-section large-file-settings">
          <h3>扫描设置</h3>
          <div className="duplicate-option-block">
            <strong>扫描磁盘</strong>
            <div className="check-grid">
              {["C:", "D:", "E:"].map((drive) => (
                <label key={drive} className="check-line">
                  <input checked={selectedDrives.includes(drive)} type="checkbox" onChange={(event) => updateDrive(drive, event.currentTarget.checked)} />
                  <span>{drive}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="duplicate-option-block">
            <strong>最小文件大小</strong>
            <div className="radio-row">
              {thresholdOptions.map((option) => (
                <label key={option.value} className="check-line">
                  <input
                    checked={minSizeBytes === option.value}
                    name="large-file-threshold"
                    type="radio"
                    onChange={() => setMinSizeBytes(option.value)}
                  />
                  <span>{option.label}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="duplicate-option-block">
            <strong>原文件处理</strong>
            <PolicyRadios originalFilePolicy={originalFilePolicy} onChange={setOriginalFilePolicy} />
          </div>
          <button className="primary-button" disabled={settings === null} type="button" onClick={() => void beginScan()}>
            {settings === null ? "正在加载设置..." : "开始扫描"}
          </button>
        </section>
      )}

      {step === "scanning" && <ProgressPanel mode="scan" progress={progress} onCancel={() => void cancelCurrentOperation()} />}

      {step === "results" && report && (
        <section className="large-file-results">
          <h3>扫描结果</h3>
          <SummaryGrid selectedBytes={selectedBytes} selectedCount={selectedCount} expectedFreedBytes={expectedFreedBytes} totalBytes={report.totalBytes} />
          <CategoryGroupList items={report.items} selectedIds={selectedIds} onToggleItem={toggleItem} />
          <div className="button-row">
            <button className="secondary-button" type="button" onClick={() => setStep("settings")}>
              返回设置
            </button>
            <button className="primary-button" disabled={selectedIds.size === 0} type="button" onClick={() => setStep("migrationSettings")}>
              设置迁移目标
            </button>
          </div>
        </section>
      )}

      {step === "migrationSettings" && report && (
        <section className="settings-section large-file-migration-settings">
          <h3>迁移目标</h3>
          <label className="inline-field large-target-field">
            <span>目标文件夹</span>
            <input value={targetFolder} onChange={(event) => setTargetFolder(event.currentTarget.value)} />
          </label>
          <p className="path-hint">建议使用非来源盘符，例如 D:\Cleaner_MigratedFiles。前端仅检查非空，后端会继续验证本地路径。</p>
          <PolicyRadios originalFilePolicy={originalFilePolicy} onChange={setOriginalFilePolicy} />
          {originalFilePolicy === "keepOriginal" && <p className="warning">选择移入回收站才会释放原位置空间</p>}
          <SummaryGrid selectedBytes={selectedBytes} selectedCount={selectedCount} expectedFreedBytes={expectedFreedBytes} totalBytes={report.totalBytes} />
          <div className="button-row">
            <button className="secondary-button" type="button" onClick={() => setStep("results")}>
              返回结果
            </button>
            <button className="primary-button" disabled={targetFolder.trim() === "" || selectedIds.size === 0} type="button" onClick={() => setStep("confirm")}>
              确认迁移内容
            </button>
          </div>
        </section>
      )}

      {step === "confirm" && report && (
        <section className="settings-section large-file-confirm">
          <h3>确认迁移</h3>
          <SummaryGrid selectedBytes={selectedBytes} selectedCount={selectedCount} expectedFreedBytes={expectedFreedBytes} totalBytes={report.totalBytes} />
          <div className="large-confirm-lines">
            <p><strong>目标文件夹</strong><span>{targetFolder}</span></p>
            <p><strong>处理阶段</strong><span>复制文件 · 校验文件 · 处理原文件</span></p>
          </div>
          <label className="danger-confirm">
            <input checked={confirmed} type="checkbox" onChange={(event) => setConfirmed(event.currentTarget.checked)} />
            <span>我已确认迁移目标、原文件处理策略和受保护文件跳过规则</span>
          </label>
          <div className="button-row">
            <button className="secondary-button" type="button" onClick={() => setStep("migrationSettings")}>
              返回目标设置
            </button>
            <button className="primary-button" disabled={!confirmed || selectedIds.size === 0 || targetFolder.trim() === ""} type="button" onClick={() => void beginMigration()}>
              <Truck size={17} />
              开始迁移
            </button>
          </div>
        </section>
      )}

      {step === "migrating" && <ProgressPanel mode="migration" progress={progress} onCancel={() => void cancelCurrentOperation()} />}

      {step === "finished" && migrationResult && (
        <section className="settings-section large-file-finished">
          <CheckCircle2 size={34} />
          <h3>迁移完成</h3>
          <div className="duplicate-summary-grid">
            <Metric label="迁移成功" value={formatBytes(migrationResult.totalCopiedBytes)} />
            <Metric label="已释放原位置空间" value={formatBytes(migrationResult.totalFreedBytes)} />
            <Metric label="迁移成功但未释放空间" value={formatBytes(Math.max(0, migrationResult.totalCopiedBytes - migrationResult.totalFreedBytes))} />
            <Metric label="失败" value={formatNumber(migrationResult.failedCount)} />
          </div>
          <ResultBuckets result={migrationResult} />
          <button className="primary-button" type="button" onClick={resetFlow}>
            再次扫描
          </button>
        </section>
      )}
    </div>
  );
}

function PolicyRadios({
  originalFilePolicy,
  onChange,
}: {
  originalFilePolicy: OriginalFilePolicy;
  onChange: (policy: OriginalFilePolicy) => void;
}) {
  return (
    <div className="radio-row">
      <label className="check-line">
        <input checked={originalFilePolicy === "keepOriginal"} name="original-policy" type="radio" onChange={() => onChange("keepOriginal")} />
        <span>保留原文件</span>
      </label>
      <label className="check-line">
        <input
          checked={originalFilePolicy === "moveOriginalToRecycleBin"}
          name="original-policy"
          type="radio"
          onChange={() => onChange("moveOriginalToRecycleBin")}
        />
        <span>迁移后将原文件移入回收站</span>
      </label>
    </div>
  );
}

function ProgressPanel({
  mode,
  progress,
  onCancel,
}: {
  mode: "scan" | "migration";
  progress: OperationProgressPayload;
  onCancel: () => void;
}) {
  const percent = Math.max(0, Math.min(100, Math.round(progress.percent)));
  return (
    <section className="settings-section duplicate-progress" aria-live="polite">
      <strong className="progress-percent">{percent}%</strong>
      <div className="progress-track" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent}>
        <div className="progress-bar" style={{ width: `${percent}%` }} />
      </div>
      <div className="duplicate-progress-grid">
        <Metric label="阶段" value={progress.stage || "准备中"} />
        <Metric label={mode === "scan" ? "当前位置" : "当前项目"} value={progress.currentLocationHint || "等待后端返回"} />
        {mode === "scan" ? (
          <>
            <Metric label="已扫描文件" value={formatNumber(progress.scannedFiles)} />
            <Metric label="发现项目" value={formatNumber(progress.foundItems)} />
            <Metric label="已发现大文件" value={formatBytes(progress.foundBytes)} />
          </>
        ) : (
          <>
            <Metric label="已处理" value={formatNumber(progress.processedItems)} />
            <Metric label="已复制" value={formatNumber(progress.successCount)} />
            <Metric label="跳过/失败" value={`${formatNumber(progress.skippedCount)} / ${formatNumber(progress.failedCount)}`} />
          </>
        )}
      </div>
      {mode === "migration" && <p className="path-hint">复制文件 · 校验文件 · 处理原文件</p>}
      <button className="secondary-button" type="button" onClick={onCancel}>
        取消
      </button>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="duplicate-metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function SummaryGrid({
  totalBytes,
  selectedBytes,
  selectedCount,
  expectedFreedBytes,
}: {
  totalBytes: number;
  selectedBytes: number;
  selectedCount: number;
  expectedFreedBytes: number;
}) {
  return (
    <div className="duplicate-summary-grid">
      <Metric label="扫描命中" value={formatBytes(totalBytes)} />
      <Metric label="已选择迁移" value={formatBytes(selectedBytes)} />
      <Metric label="预计释放原位置空间" value={formatBytes(expectedFreedBytes)} />
      <Metric label="选中文件数" value={formatNumber(selectedCount)} />
    </div>
  );
}

function CategoryGroupList({
  items,
  selectedIds,
  onToggleItem,
}: {
  items: LargeFileItem[];
  selectedIds: Set<string>;
  onToggleItem: (item: LargeFileItem, checked: boolean) => void;
}) {
  return (
    <div className="large-category-list">
      {categoryOrder.map((category) => {
        const categoryItems = items.filter((item) => item.category === category);
        return (
          <section key={category} className="large-category-group">
            <h3>{categoryLabels[category]}</h3>
            {categoryItems.length === 0 && <p className="tool-status">未发现此类大文件</p>}
            {categoryItems.map((item) => (
              <div key={item.itemId} className="duplicate-file-row large-file-row">
                <label className="check-line duplicate-file-check">
                  <input
                    aria-label={`${item.displayName} ${item.visibleLocationHint}${item.protected ? " 受保护" : ""}`}
                    checked={selectedIds.has(item.itemId)}
                    disabled={item.protected}
                    type="checkbox"
                    onChange={(event) => onToggleItem(item, event.currentTarget.checked)}
                  />
                  <span>{item.displayName}</span>
                </label>
                <span className="path-hint">{item.visibleLocationHint}</span>
                <span className="bytes">{formatBytes(item.sizeBytes)}</span>
                {item.protected ? (
                  <span className="duplicate-protected">
                    <ShieldCheck size={14} />
                    受保护
                  </span>
                ) : item.recommended ? (
                  <span className="risk-badge" data-risk="recommended">建议迁移</span>
                ) : (
                  <span className="risk-badge" data-risk="optional">可选</span>
                )}
              </div>
            ))}
          </section>
        );
      })}
    </div>
  );
}

function ResultBuckets({ result }: { result: MigrationResult }) {
  const copiedOnly = result.itemResults.filter((item) => item.status === "copied");
  const failed = result.itemResults.filter((item) => item.status === "failed");
  return (
    <div className="large-result-buckets">
      <p><strong>迁移成功</strong><span>{formatNumber(result.copiedCount)} 个项目</span></p>
      <p><strong>迁移成功但未释放空间</strong><span>{formatBytes(copiedOnly.reduce((sum, item) => sum + item.bytesCopied, 0))}</span></p>
      <p><strong>失败</strong><span>{formatNumber(failed.length)} 个项目</span></p>
      {failed.length > 0 && <XCircle size={18} aria-hidden="true" />}
    </div>
  );
}

function buildDefaultSelection(report: LargeFileScanReport): Set<string> {
  return new Set(report.items.filter((item) => item.recommended && !item.protected).map((item) => item.itemId));
}

function buildMigrationRequest(
  report: LargeFileScanReport,
  selectedIds: Set<string>,
  targetFolder: string,
  originalFilePolicy: OriginalFilePolicy,
): MigrationRequest {
  return {
    selectedItemIds: report.items.filter((item) => selectedIds.has(item.itemId) && !item.protected).map((item) => item.itemId),
    scanReport: report,
    targetFolder: targetFolder.trim(),
    originalFilePolicy,
    protectedOverrideConfirmed: false,
  };
}

function suggestTargetFolder(selectedDrives: string[]): string {
  const sourceDrives = new Set(selectedDrives.map((drive) => drive.toUpperCase()));
  const targetDrive = ["D:", "E:", "F:"].find((drive) => !sourceDrives.has(drive));
  return targetDrive ? `${targetDrive}\\Cleaner_MigratedFiles` : "";
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN").format(value);
}
