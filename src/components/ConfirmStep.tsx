import type { ScanItem } from "../domain/models";
import { estimateSelectedBytes, requiresHighRiskConfirmation } from "../domain/selection";

export function ConfirmStep({
  items,
  selectedIds,
  highRiskConfirmed,
  onHighRiskConfirmed,
  onBack,
  onConfirm,
}: {
  items: ScanItem[];
  selectedIds: string[];
  highRiskConfirmed: boolean;
  onHighRiskConfirmed: (confirmed: boolean) => void;
  onBack: () => void;
  onConfirm: () => void;
}) {
  const needsHighRisk = requiresHighRiskConfirmation(selectedIds, items);
  const selected = new Set(selectedIds);
  const selectedItems = items.filter((item) => selectedIds.includes(item.id));
  const untouchedItems = items.filter((item) => !selected.has(item.id));
  const highRiskItems = selectedItems.filter((item) => item.riskLevel === "highRisk");
  const canConfirm = selectedIds.length > 0 && (!needsHighRisk || highRiskConfirmed);

  return (
    <div className="step-content">
      <p className="eyebrow">确认清理</p>
      <h2>预计释放 {formatBytes(estimateSelectedBytes(selectedIds, items))}</h2>
      <div className="confirm-summary-grid">
        <div>
          <span>已选清理 {selectedItems.length} 项</span>
          <strong>{formatBytes(estimateSelectedBytes(selectedIds, items))}</strong>
        </div>
        <div>
          <span>不会触碰 {untouchedItems.length} 项</span>
          <strong>保持原样</strong>
        </div>
      </div>
      <div className="confirm-list">
        {selectedItems.map((item) => (
          <div className="confirm-row" key={item.id}>
            <span>{item.title}</span>
            <strong>{formatBytes(item.estimatedBytes)}</strong>
          </div>
        ))}
      </div>
      {highRiskItems.length > 0 && (
        <section className="high-risk-summary">
          <h3>高风险项目</h3>
          <div className="confirm-list">
            {highRiskItems.map((item) => (
              <div className="confirm-row" key={item.id}>
                <span>{item.title}</span>
                <strong>{formatBytes(item.estimatedBytes)}</strong>
              </div>
            ))}
          </div>
        </section>
      )}
      {needsHighRisk && (
        <label className="danger-confirm">
          <input
            checked={highRiskConfirmed}
            onChange={(event) => onHighRiskConfirmed(event.target.checked)}
            type="checkbox"
          />
          我理解高风险项目可能删除聊天文件、本地文件或离线数据，删除后无法保证恢复。
        </label>
      )}
      <div className="button-row">
        <button className="secondary-button" onClick={onBack}>
          返回修改
        </button>
        <button className="primary-button" disabled={!canConfirm} onClick={onConfirm}>
          确认清理已选项目
        </button>
      </div>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
