import { CheckCircle2, ChevronDown, ChevronRight, ShieldCheck, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CleanerSettings,
  DuplicateCleanupReport,
  DuplicateCleanupRequest,
  DuplicateDefaultStrategy,
  DuplicateFileEntry,
  DuplicateFileGroup,
  DuplicateFileType,
  DuplicateScanReport,
  DuplicateScanRequest,
  OperationFinishedPayload,
  OperationProgressPayload,
} from "../../domain/v2";
import {
  cancelOperation,
  getCleanerSettings,
  onCleanerOperationFinished,
  onCleanerOperationProgress,
  startDuplicateCleanup,
  startDuplicateScan,
} from "../../services/v2Api";

export type DuplicateStep = "settings" | "scanning" | "results" | "confirm" | "cleaning" | "finished";
type CleanerUnsubscribe = () => Promise<void>;
type DuplicateCleanerPageProps = {
  onBlockingWorkChange?: (blocking: boolean) => void;
};

const fileTypeOptions: Array<{ value: DuplicateFileType; label: string }> = [
  { value: "image", label: "图片" },
  { value: "document", label: "文档" },
  { value: "archive", label: "压缩包" },
  { value: "audio", label: "音频" },
  { value: "video", label: "视频" },
];

const strategyLabels: Array<{ value: DuplicateDefaultStrategy | "manual"; label: string }> = [
  { value: "cDriveFirstKeepNewest", label: "C 盘优先" },
  { value: "keepNewest", label: "保留最新" },
  { value: "keepOldest", label: "保留最旧" },
  { value: "manual", label: "手动选择" },
];

const emptyProgress: OperationProgressPayload = {
  operationId: "",
  module: "duplicateScan",
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

export function DuplicateCleanerPage({ onBlockingWorkChange }: DuplicateCleanerPageProps = {}) {
  const [step, setStep] = useState<DuplicateStep>("settings");
  const [settings, setSettings] = useState<CleanerSettings | null>(null);
  const [fileTypes, setFileTypes] = useState<DuplicateFileType[]>(["image", "document", "archive"]);
  const [selectedDrives, setSelectedDrives] = useState(["C:"]);
  const [customFolders, setCustomFolders] = useState<string[]>([]);
  const [customFolderInput, setCustomFolderInput] = useState("");
  const [includeSuspected, setIncludeSuspected] = useState(false);
  const [minSizeBytes, setMinSizeBytes] = useState(1);
  const [strategy, setStrategy] = useState<DuplicateDefaultStrategy | "manual">("cDriveFirstKeepNewest");
  const [report, setReport] = useState<DuplicateScanReport | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [expandedSuspected, setExpandedSuspected] = useState<Set<string>>(new Set());
  const [confirmed, setConfirmed] = useState(false);
  const [progress, setProgress] = useState<OperationProgressPayload>(emptyProgress);
  const [cleanupResult, setCleanupResult] = useState<DuplicateCleanupReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const operationIdRef = useRef<string | null>(null);
  const mountedRef = useRef(true);
  const unsubscribeProgressRef = useRef<CleanerUnsubscribe | null>(null);
  const unsubscribeFinishedRef = useRef<CleanerUnsubscribe | null>(null);
  const pendingProgressRef = useRef<OperationProgressPayload[]>([]);
  const pendingFinishedRef = useRef<OperationFinishedPayload[]>([]);

  async function cleanupActiveOperation(cancelBackend: boolean) {
    const operationId = operationIdRef.current;
    const unsubscribeProgress = unsubscribeProgressRef.current;
    const unsubscribeFinished = unsubscribeFinishedRef.current;
    operationIdRef.current = null;
    unsubscribeProgressRef.current = null;
    unsubscribeFinishedRef.current = null;
    pendingProgressRef.current = [];
    pendingFinishedRef.current = [];

    await Promise.allSettled([
      unsubscribeProgress?.(),
      unsubscribeFinished?.(),
      cancelBackend && operationId ? cancelOperation(operationId) : Promise.resolve(),
    ]);
  }

  function safelySetError(message: string) {
    if (mountedRef.current) setError(message);
  }

  function safelySetStep(nextStep: DuplicateStep) {
    if (mountedRef.current) setStep(nextStep);
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
        setStrategy(nextSettings.duplicateDefaultStrategy);
      })
      .catch(() => {
        if (disposed) return;
        setSettings(defaultSettings());
        setError("设置加载失败，已使用默认重复文件扫描设置。");
      });
    return () => {
      disposed = true;
    };
  }, []);

  const allGroups = useMemo(() => {
    if (!report) return [];
    return [...report.strictGroups, ...report.suspectedGroups];
  }, [report]);

  const selectedFiles = useMemo(() => {
    return allGroups.flatMap((group) => group.files.filter((file) => selectedIds.has(file.entryId)));
  }, [allGroups, selectedIds]);

  const selectedBytes = selectedFiles.reduce((sum, file) => sum + file.sizeBytes, 0);
  const cDriveSelectedBytes = selectedFiles
    .filter((file) => file.drive.toUpperCase().startsWith("C"))
    .reduce((sum, file) => sum + file.sizeBytes, 0);
  const otherDriveSelectedBytes = selectedBytes - cDriveSelectedBytes;

  useEffect(() => {
    const hasUnfinishedSelection = (step === "results" || step === "confirm") && selectedIds.size > 0;
    const hasUnfinishedResults = (step === "results" || step === "confirm") && report !== null;
    onBlockingWorkChange?.(step === "scanning" || step === "cleaning" || hasUnfinishedSelection || hasUnfinishedResults);
  }, [onBlockingWorkChange, report, selectedIds.size, step]);

  useEffect(() => {
    return () => {
      onBlockingWorkChange?.(false);
    };
  }, [onBlockingWorkChange]);

  function updateFileType(type: DuplicateFileType, checked: boolean) {
    const nextTypes = checked ? [...fileTypes, type] : fileTypes.filter((item) => item !== type);
    setFileTypes(nextTypes.length ? Array.from(new Set(nextTypes)) : ["image"]);
  }

  function updateDrive(drive: string, checked: boolean) {
    const nextDrives = checked ? [...selectedDrives, drive] : selectedDrives.filter((item) => item !== drive);
    setSelectedDrives(nextDrives.length ? Array.from(new Set(nextDrives)) : ["C:"]);
  }

  function addCustomFolder() {
    const folder = customFolderInput.trim();
    if (!folder) return;
    setCustomFolders((current) => Array.from(new Set([...current, folder])));
    setCustomFolderInput("");
  }

  function removeCustomFolder(folder: string) {
    setCustomFolders((current) => current.filter((item) => item !== folder));
  }

  function handleProgressEvent(expectedModule: OperationProgressPayload["module"], payload: OperationProgressPayload) {
    if (!mountedRef.current || payload.module !== expectedModule) return;
    const operationId = operationIdRef.current;
    if (!operationId) {
      pendingProgressRef.current = [...pendingProgressRef.current, payload];
      return;
    }
    if (payload.operationId === operationId) {
      setProgress(payload);
    }
  }

  function handleFinishedEvent(expectedModule: OperationFinishedPayload["module"], payload: OperationFinishedPayload) {
    if (!mountedRef.current || payload.module !== expectedModule) return;
    const operationId = operationIdRef.current;
    if (!operationId) {
      pendingFinishedRef.current = [...pendingFinishedRef.current, payload];
      return;
    }
    if (payload.operationId === operationId) {
      void cleanupActiveOperation(false);
      if (expectedModule === "duplicateScan") {
        handleScanFinished(payload as OperationFinishedPayload<DuplicateScanReport>);
      } else if (expectedModule === "duplicateCleanup") {
        handleCleanupFinished(payload as OperationFinishedPayload<DuplicateCleanupReport>);
      }
    }
  }

  function flushPendingEvents(
    expectedModule: OperationFinishedPayload["module"],
    operationId: string,
    onFinished: (payload: OperationFinishedPayload) => void,
  ) {
    const matchingProgress = pendingProgressRef.current.filter((payload) => payload.module === expectedModule && payload.operationId === operationId);
    const matchingFinished = pendingFinishedRef.current.find((payload) => payload.module === expectedModule && payload.operationId === operationId);
    pendingProgressRef.current = pendingProgressRef.current.filter((payload) => payload.module !== expectedModule);
    pendingFinishedRef.current = pendingFinishedRef.current.filter((payload) => payload.module !== expectedModule);

    matchingProgress.forEach((payload) => setProgress(payload));
    if (matchingFinished) {
      void cleanupActiveOperation(false);
      onFinished(matchingFinished);
    }
  }

  async function beginScan() {
    if (!settings) {
      setError("设置仍在加载中，请稍后再开始扫描。");
      return;
    }

    const request: DuplicateScanRequest = {
      selectedDrives,
      customFolders,
      fileTypes,
      customExtensions: [],
      includeSuspected,
      minSizeBytes: Math.max(1, Math.trunc(minSizeBytes)),
      protectedPaths: settings.protectedPaths,
    };
    operationIdRef.current = null;
    setError(null);
    setProgress({ ...emptyProgress, module: "duplicateScan", stage: "正在启动扫描" });
    setStep("scanning");

    try {
      unsubscribeProgressRef.current = await onCleanerOperationProgress((payload) => {
        handleProgressEvent("duplicateScan", payload);
      });
      unsubscribeFinishedRef.current = await onCleanerOperationFinished((payload) => {
        handleFinishedEvent("duplicateScan", payload);
      });
      const operation = await startDuplicateScan(request);
      if (!mountedRef.current) {
        await cancelOperation(operation.operationId);
        return;
      }
      operationIdRef.current = operation.operationId;
      flushPendingEvents("duplicateScan", operation.operationId, (payload) => handleScanFinished(payload as OperationFinishedPayload<DuplicateScanReport>));
    } catch {
      await cleanupActiveOperation(false);
      safelySetError("重复文件扫描启动失败，本次未执行任何清理。");
      safelySetStep("settings");
    }
  }

  function handleScanFinished(payload: OperationFinishedPayload<DuplicateScanReport>) {
    operationIdRef.current = null;
    if (payload.status === "completed" && payload.result) {
      setReport(payload.result);
      setSelectedIds(buildSelection(payload.result, strategy));
      setConfirmed(false);
      setExpandedSuspected(new Set());
      setStep("results");
      return;
    }
    setError(payload.status === "cancelled" ? "扫描已取消。" : payload.message || "扫描失败，请稍后重试。");
    setStep("settings");
  }

  async function beginCleanup() {
    if (!report) return;
    operationIdRef.current = null;
    setError(null);
    setProgress({ ...emptyProgress, module: "duplicateCleanup", stage: "正在启动清理" });
    setStep("cleaning");

    try {
      unsubscribeProgressRef.current = await onCleanerOperationProgress((payload) => {
        handleProgressEvent("duplicateCleanup", payload);
      });
      unsubscribeFinishedRef.current = await onCleanerOperationFinished((payload) => {
        handleFinishedEvent("duplicateCleanup", payload);
      });
      const operation = await startDuplicateCleanup(buildCleanupRequest(report, selectedIds));
      if (!mountedRef.current) {
        await cancelOperation(operation.operationId);
        return;
      }
      operationIdRef.current = operation.operationId;
      flushPendingEvents("duplicateCleanup", operation.operationId, (payload) => handleCleanupFinished(payload as OperationFinishedPayload<DuplicateCleanupReport>));
    } catch {
      await cleanupActiveOperation(false);
      safelySetError("清理启动失败，本次没有删除任何文件。");
      safelySetStep("confirm");
    }
  }

  function handleCleanupFinished(payload: OperationFinishedPayload<DuplicateCleanupReport>) {
    operationIdRef.current = null;
    if (payload.status === "completed" && payload.result) {
      setCleanupResult(payload.result);
      setStep("finished");
      return;
    }
    setError(payload.status === "cancelled" ? "清理已取消。" : payload.message || "清理失败，本次没有永久删除文件。");
    setStep("confirm");
  }

  async function cancelCurrentOperation() {
    if (!operationIdRef.current) return;
    await cancelOperation(operationIdRef.current);
  }

  function applyStrategy(nextStrategy: DuplicateDefaultStrategy | "manual") {
    setStrategy(nextStrategy);
    if (!report || nextStrategy === "manual") return;
    setSelectedIds(buildSelection(report, nextStrategy));
  }

  function toggleFile(group: DuplicateFileGroup, file: DuplicateFileEntry, checked: boolean) {
    if (file.protected) return;
    setStrategy("manual");
    setSelectedIds((current) => {
      const next = new Set(current);
      if (checked && canSelectMoreInGroup(group, file.entryId, next)) {
        next.add(file.entryId);
      } else {
        next.delete(file.entryId);
      }
      return next;
    });
  }

  function toggleGroup(group: DuplicateFileGroup, checked: boolean) {
    setStrategy("manual");
    setSelectedIds((current) => {
      const next = new Set(current);
      group.files.forEach((file) => next.delete(file.entryId));
      if (!checked) return next;
      selectCandidatesForGroup(group, group.files.filter((file) => !file.protected), next);
      return next;
    });
  }

  return (
    <div className="tool-page duplicate-page">
      <header className="tool-header duplicate-header">
        <div>
          <p className="eyebrow">Duplicate Cleaner</p>
          <h2>重复文件清理</h2>
        </div>
        {error && <p className="duplicate-error">{error}</p>}
      </header>

      {step === "settings" && (
        <section className="settings-section duplicate-settings">
          <h3>扫描设置</h3>
          <div className="duplicate-option-block">
            <strong>文件类型</strong>
            <div className="check-grid">
              {fileTypeOptions.map((option) => (
                <label key={option.value} className="check-line">
                  <input
                    checked={fileTypes.includes(option.value)}
                    type="checkbox"
                    onChange={(event) => updateFileType(option.value, event.currentTarget.checked)}
                  />
                  <span>{option.label}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="duplicate-option-block">
            <strong>扫描磁盘</strong>
            <div className="check-grid">
              {["C:", "D:", "E:"].map((drive) => (
                <label key={drive} className="check-line">
                  <input
                    checked={selectedDrives.includes(drive)}
                    type="checkbox"
                    onChange={(event) => updateDrive(drive, event.currentTarget.checked)}
                  />
                  <span>{drive}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="duplicate-option-block">
            <strong>自定义文件夹</strong>
            <div className="folder-input-row">
              <label className="inline-field">
                <span>文件夹路径</span>
                <input
                  value={customFolderInput}
                  placeholder="例如 D:\\Downloads"
                  onChange={(event) => setCustomFolderInput(event.currentTarget.value)}
                />
              </label>
              <button className="secondary-button" type="button" onClick={addCustomFolder}>
                添加文件夹
              </button>
            </div>
            {customFolders.length > 0 && (
              <div className="folder-chip-list">
                {customFolders.map((folder) => (
                  <span key={folder} className="folder-chip">
                    {folder}
                    <button type="button" aria-label={`移除 ${folder}`} onClick={() => removeCustomFolder(folder)}>
                      移除
                    </button>
                  </span>
                ))}
              </div>
            )}
          </div>
          <label className="check-line">
            <input
              checked={includeSuspected}
              type="checkbox"
              onChange={(event) => setIncludeSuspected(event.currentTarget.checked)}
            />
            <span>包含疑似重复文件</span>
          </label>
          <label className="inline-field duplicate-size-field">
            <span>最小文件大小（字节）</span>
            <input
              min={1}
              type="number"
              value={minSizeBytes}
              onChange={(event) => setMinSizeBytes(Math.max(1, Math.trunc(Number(event.currentTarget.value) || 1)))}
            />
          </label>
          <button className="primary-button" disabled={settings === null} type="button" onClick={() => void beginScan()}>
            {settings === null ? "正在加载设置..." : "开始扫描"}
          </button>
        </section>
      )}

      {step === "scanning" && (
        <ProgressPanel
          mode="scan"
          progress={progress}
          onCancel={() => void cancelCurrentOperation()}
        />
      )}

      {step === "results" && report && (
        <section className="duplicate-results">
          <h3>扫描结果</h3>
          <SummaryGrid
            total={report.totalReclaimableBytes}
            selected={selectedBytes}
            cDrive={cDriveSelectedBytes}
            otherDrive={otherDriveSelectedBytes}
          />
          <div className="segmented duplicate-strategies">
            {strategyLabels.map((item) => (
              <button
                key={item.value}
                data-active={strategy === item.value}
                type="button"
                onClick={() => applyStrategy(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
          <DuplicateGroupList
            groups={report.strictGroups}
            selectedIds={selectedIds}
            title="严格重复"
            onToggleFile={toggleFile}
            onToggleGroup={toggleGroup}
          />
          <DuplicateGroupList
            collapsible
            expandedIds={expandedSuspected}
            groups={report.suspectedGroups}
            readOnly
            selectedIds={selectedIds}
            title="疑似重复"
            onExpand={(groupId) =>
              setExpandedSuspected((current) => {
                const next = new Set(current);
                if (next.has(groupId)) next.delete(groupId);
                else next.add(groupId);
                return next;
              })
            }
            onToggleFile={toggleFile}
            onToggleGroup={toggleGroup}
          />
          <div className="button-row">
            <button className="secondary-button" type="button" onClick={() => setStep("settings")}>
              返回设置
            </button>
            <button className="primary-button" disabled={selectedIds.size === 0} type="button" onClick={() => setStep("confirm")}>
              下一步
            </button>
          </div>
        </section>
      )}

      {step === "confirm" && report && (
        <section className="settings-section duplicate-confirm">
          <h3>确认清理</h3>
          <SummaryGrid total={report.totalReclaimableBytes} selected={selectedBytes} cDrive={cDriveSelectedBytes} otherDrive={otherDriveSelectedBytes} />
          <ul className="duplicate-confirm-copy">
            <li>文件将移入回收站</li>
            <li>每个重复组至少保留 1 份</li>
            <li>保护路径不会被自动清理</li>
            <li>回收站失败时不会永久删除</li>
          </ul>
          <label className="danger-confirm">
            <input checked={confirmed} type="checkbox" onChange={(event) => setConfirmed(event.currentTarget.checked)} />
            <span>我已确认上述清理规则</span>
          </label>
          <div className="button-row">
            <button className="secondary-button" type="button" onClick={() => setStep("results")}>
              返回结果
            </button>
            <button className="primary-button" disabled={!confirmed || selectedIds.size === 0} type="button" onClick={() => void beginCleanup()}>
              <Trash2 size={17} />
              开始清理
            </button>
          </div>
        </section>
      )}

      {step === "cleaning" && <ProgressPanel mode="cleanup" progress={progress} onCancel={() => void cancelCurrentOperation()} />}

      {step === "finished" && cleanupResult && (
        <section className="settings-section duplicate-finished">
          <CheckCircle2 size={34} />
          <h3>清理完成</h3>
          <p>已释放 {formatBytes(cleanupResult.freedBytes)}</p>
          <ResultCounts success={cleanupResult.successCount} skipped={cleanupResult.skippedCount} failed={cleanupResult.failedCount} />
          <button className="primary-button" type="button" onClick={() => {
            setReport(null);
            setSelectedIds(new Set());
            setCleanupResult(null);
            setConfirmed(false);
            setStep("settings");
          }}>
            再次扫描
          </button>
        </section>
      )}
    </div>
  );
}

function ProgressPanel({
  mode,
  progress,
  onCancel,
}: {
  mode: "scan" | "cleanup";
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
            <Metric label="发现重复组" value={formatNumber(progress.foundGroups)} />
            <Metric label="发现文件" value={formatNumber(progress.foundItems)} />
          </>
        ) : (
          <>
            <Metric label="已处理" value={formatNumber(progress.processedItems)} />
            <ResultCounts success={progress.successCount} skipped={progress.skippedCount} failed={progress.failedCount} />
          </>
        )}
      </div>
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

function ResultCounts({ success, skipped, failed }: { success: number; skipped: number; failed: number }) {
  return (
    <div className="duplicate-counts">
      <span>已移入回收站：{success}</span>
      <span>已跳过：{skipped}</span>
      <span>失败：{failed}</span>
    </div>
  );
}

function SummaryGrid({ total, selected, cDrive, otherDrive }: { total: number; selected: number; cDrive: number; otherDrive: number }) {
  return (
    <div className="duplicate-summary-grid">
      <Metric label="可释放" value={formatBytes(total)} />
      <Metric label="已选择" value={formatBytes(selected)} />
      <Metric label="C 盘已选" value={formatBytes(cDrive)} />
      <Metric label="其他盘已选" value={formatBytes(otherDrive)} />
    </div>
  );
}

function DuplicateGroupList({
  title,
  groups,
  selectedIds,
  readOnly = false,
  collapsible = false,
  expandedIds,
  onExpand,
  onToggleFile,
  onToggleGroup,
}: {
  title: string;
  groups: DuplicateFileGroup[];
  selectedIds: Set<string>;
  readOnly?: boolean;
  collapsible?: boolean;
  expandedIds?: Set<string>;
  onExpand?: (groupId: string) => void;
  onToggleFile: (group: DuplicateFileGroup, file: DuplicateFileEntry, checked: boolean) => void;
  onToggleGroup: (group: DuplicateFileGroup, checked: boolean) => void;
}) {
  return (
    <section className="duplicate-group-section">
      <h3>{title}</h3>
      {groups.length === 0 && <p className="tool-status">未发现{title}</p>}
      {groups.map((group) => {
        const expanded = !collapsible || expandedIds?.has(group.groupId);
        const cleanableFiles = group.files.filter((file) => !file.protected);
        const groupChecked = cleanableFiles.some((file) => selectedIds.has(file.entryId));
        return (
          <article key={group.groupId} className="duplicate-group" data-testid={`duplicate-group-${group.groupId}`}>
            <div className="duplicate-group-head">
              <label className="check-line">
                <input
                  checked={groupChecked}
                  disabled={readOnly || cleanableFiles.length <= 1}
                  type="checkbox"
                  onChange={(event) => onToggleGroup(group, event.currentTarget.checked)}
                />
                <span>{group.files.length} 个文件 · 可释放 {formatBytes(group.reclaimableBytes)}</span>
              </label>
              {collapsible && (
                <button className="secondary-button duplicate-expand" type="button" onClick={() => onExpand?.(group.groupId)}>
                  {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                  {expanded ? "收起疑似重复" : "展开疑似重复"}
                </button>
              )}
            </div>
            <p className="reason">{group.recommendedSelectionReason}</p>
            {readOnly && <p className="reason">疑似重复仅供查看，不会加入清理。</p>}
            {expanded && (
              <div className="duplicate-file-list">
                {group.files.map((file) => (
                  <div key={file.entryId} className="duplicate-file-row">
                    <label className="check-line duplicate-file-check">
                      <input
                        aria-label={`${file.displayName} ${file.visibleLocationHint}${file.protected ? " 受保护" : ""}`}
                        checked={selectedIds.has(file.entryId)}
                        disabled={readOnly || file.protected}
                        type="checkbox"
                        onChange={(event) => onToggleFile(group, file, event.currentTarget.checked)}
                      />
                      <span>{file.displayName}</span>
                    </label>
                    <span className="path-hint">{file.visibleLocationHint}</span>
                    <span className="bytes">{formatBytes(file.sizeBytes)}</span>
                    {file.protected && (
                      <span className="duplicate-protected">
                        <ShieldCheck size={14} />
                        受保护
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </article>
        );
      })}
    </section>
  );
}

function buildSelection(report: DuplicateScanReport, strategy: DuplicateDefaultStrategy | "manual"): Set<string> {
  const selected = new Set<string>();
  if (strategy === "manual") return selected;
  report.strictGroups.forEach((group) => {
    const ranked = rankFiles(group.files.filter((file) => !file.protected), strategy);
    selectCandidatesForGroup(group, ranked.slice(1), selected);
  });
  return selected;
}

function rankFiles(files: DuplicateFileEntry[], strategy: DuplicateDefaultStrategy): DuplicateFileEntry[] {
  return [...files].sort((a, b) => {
    if (strategy === "cDriveFirstKeepNewest") {
      const driveRank = drivePriority(a) - drivePriority(b);
      if (driveRank !== 0) return driveRank;
    }
    const timeA = Date.parse(a.modifiedAt);
    const timeB = Date.parse(b.modifiedAt);
    return strategy === "keepOldest" ? timeA - timeB : timeB - timeA;
  });
}

function selectCandidatesForGroup(group: DuplicateFileGroup, candidates: DuplicateFileEntry[], selected: Set<string>) {
  const cleanableCount = group.files.filter((file) => !file.protected).length;
  const limit = Math.max(0, cleanableCount - 1);
  candidates.slice(0, limit).forEach((file) => {
    if (!file.protected) selected.add(file.entryId);
  });
}

function canSelectMoreInGroup(group: DuplicateFileGroup, nextFileId: string, selected: Set<string>): boolean {
  const selectedInGroup = group.files.filter((file) => selected.has(file.entryId) || file.entryId === nextFileId).length;
  return selectedInGroup < group.files.length;
}

function drivePriority(file: DuplicateFileEntry): number {
  return file.drive.toUpperCase().startsWith("C") ? 1 : 0;
}

function buildCleanupRequest(report: DuplicateScanReport, selectedIds: Set<string>): DuplicateCleanupRequest {
  return {
    protectedOverrideConfirmed: false,
    groups: report.strictGroups
      .map((group) => ({
        groupId: group.groupId,
        files: group.files.map((file) => ({
          entryId: file.entryId,
          selected: selectedIds.has(file.entryId) && !file.protected,
          protected: file.protected,
        })),
      }))
      .filter((group) => group.files.some((file) => file.selected)),
  };
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

function defaultSettings(): CleanerSettings {
  return {
    protectedPaths: [],
    defaultScanDrives: ["C:"],
    duplicateDefaultStrategy: "cDriveFirstKeepNewest",
    largeFileDefaultThresholdBytes: 500 * 1024 * 1024,
    historyRetentionDays: 30,
    desktopShortcutEnabled: false,
    cDriveContextMenuEnabled: false,
    scheduledScanReminderEnabled: false,
  };
}
