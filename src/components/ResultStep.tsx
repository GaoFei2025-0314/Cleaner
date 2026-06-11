import type { CleanupResult } from "../domain/models";

export function ResultStep({ result, onRestart }: { result: CleanupResult; onRestart: () => void }) {
  return (
    <div className="step-content">
      <p className="eyebrow">完成</p>
      <h2>实际释放 {formatBytes(result.totalFreedBytes)}</h2>
      <div className="confirm-list">
        {result.results.map((item) => (
          <div className="confirm-row" key={item.itemId}>
            <span>{item.message}</span>
            <strong>{formatBytes(item.freedBytes)}</strong>
          </div>
        ))}
      </div>
      <button className="primary-button" onClick={onRestart}>
        重新扫描
      </button>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
