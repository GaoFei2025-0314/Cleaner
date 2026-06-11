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
  const selectedItems = items.filter((item) => selectedIds.includes(item.id));
  const canConfirm = selectedIds.length > 0 && (!needsHighRisk || highRiskConfirmed);

  return (
    <div className="step-content">
      <p className="eyebrow">确认清理</p>
      <h2>预计释放 {formatBytes(estimateSelectedBytes(selectedIds, items))}</h2>
      <div className="confirm-list">
        {selectedItems.map((item) => (
          <div className="confirm-row" key={item.id}>
            <span>{item.title}</span>
            <strong>{formatBytes(item.estimatedBytes)}</strong>
          </div>
        ))}
      </div>
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
