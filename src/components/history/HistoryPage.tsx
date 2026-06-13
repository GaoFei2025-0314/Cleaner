import { Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { HistoryEntry, OperationModule } from "../../domain/v2";
import { clearOperationHistory, listOperationHistory } from "../../services/v2Api";

type HistoryFilter = "cDrive" | "duplicate" | "largeFiles";

const moduleLabels: Record<OperationModule, string> = {
  cDriveCleanup: "C 盘清理",
  duplicateScan: "重复文件扫描",
  duplicateCleanup: "重复文件清理",
  largeFileScan: "大文件扫描",
  largeFileMigration: "大文件迁移",
};

const filterButtons: Array<{ id: HistoryFilter; label: string }> = [
  { id: "cDrive", label: "C 盘清理" },
  { id: "duplicate", label: "重复文件清理" },
  { id: "largeFiles", label: "大文件迁移" },
];

export function HistoryPage() {
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [filter, setFilter] = useState<HistoryFilter>("duplicate");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    void listOperationHistory().then((entries) => {
      if (cancelled) return;
      setHistory(entries);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  const filteredHistory = useMemo(
    () =>
      history.filter((entry) => {
        if (filter === "cDrive") return entry.module === "cDriveCleanup";
        if (filter === "duplicate") return entry.module === "duplicateCleanup" || entry.module === "duplicateScan";
        return entry.module === "largeFileMigration" || entry.module === "largeFileScan";
      }),
    [filter, history],
  );

  async function clearHistory() {
    await clearOperationHistory();
    setHistory([]);
  }

  return (
    <div className="tool-page history-page">
      <header className="tool-header">
        <div>
          <p className="eyebrow">History</p>
          <h2>清理历史</h2>
        </div>
        <button className="secondary-button" type="button" onClick={() => void clearHistory()}>
          <Trash2 size={17} />
          清空历史
        </button>
      </header>

      <div className="segmented history-tabs" role="tablist" aria-label="历史模块筛选">
        {filterButtons.map((button) => (
          <button
            key={button.id}
            aria-selected={filter === button.id}
            data-active={filter === button.id}
            role="tab"
            type="button"
            onClick={() => setFilter(button.id)}
          >
            {button.label}
          </button>
        ))}
      </div>

      {loading ? (
        <p className="tool-status">正在加载清理历史...</p>
      ) : filteredHistory.length === 0 ? (
        <p className="empty-history">暂无清理历史</p>
      ) : (
        <div className="history-table-wrap">
          <table className="history-table">
            <thead>
              <tr>
                <th>时间</th>
                <th>模块</th>
                <th>释放/迁移</th>
                <th>成功</th>
                <th>跳过</th>
                <th>失败</th>
              </tr>
            </thead>
            <tbody>
              {filteredHistory.map((entry) => (
                <tr key={entry.historyId}>
                  <td>{formatDateTime(entry.finishedAt)}</td>
                  <td>{moduleLabels[entry.module]}</td>
                  <td>{formatBytes(entry.freedBytes || entry.totalBytes)}</td>
                  <td>{entry.successCount}</td>
                  <td>{entry.skippedCount}</td>
                  <td>{entry.failedCount}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
