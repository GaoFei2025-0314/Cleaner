import { AlertTriangle, LockKeyhole, ShieldCheck } from "lucide-react";
import type { ScanItem } from "../domain/models";
import { RiskBadge } from "./RiskBadge";

export function CleanupItemRow({
  item,
  checked,
  onCheckedChange,
}: {
  item: ScanItem;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  const selectable = item.riskLevel !== "notCleanable" && item.cleanupAction === "directDelete";
  return (
    <article className="cleanup-item" data-disabled={!selectable}>
      <label className="check-cell">
        <input
          checked={selectable && checked}
          disabled={!selectable}
          onChange={(event) => onCheckedChange(event.target.checked)}
          type="checkbox"
        />
      </label>
      <div className="item-status-icon">
        {item.riskLevel === "recommended" && <ShieldCheck size={18} />}
        {item.riskLevel === "highRisk" && <AlertTriangle size={18} />}
        {item.riskLevel === "notCleanable" && <LockKeyhole size={18} />}
        {item.riskLevel === "optional" && <ShieldCheck size={18} />}
      </div>
      <div className="cleanup-main">
        <div className="cleanup-title-line">
          <h3>{item.title}</h3>
          <RiskBadge risk={item.riskLevel} />
          {item.cleanupAction === "blockedByConfigReference" && <span className="action-badge">配置保护</span>}
          {item.cleanupAction === "blockedByProcess" && <span className="action-badge">正在使用</span>}
        </div>
        <p>{item.description}</p>
        <p className="path-hint">{item.userVisiblePathHint}</p>
        {item.reasons.map((reason) => (
          <p className="reason" key={reason}>
            {reason}
          </p>
        ))}
        {item.warnings.map((warning) => (
          <p className="warning" key={warning}>
            {warning}
          </p>
        ))}
      </div>
      <strong className="bytes">{formatBytes(item.estimatedBytes)}</strong>
    </article>
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
