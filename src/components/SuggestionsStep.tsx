import { HardDrive, Sparkles } from "lucide-react";
import type { ScanItem } from "../domain/models";
import { groupByRisk, groupBySource } from "../domain/grouping";
import { estimateSelectedBytes, toggleSelection } from "../domain/selection";
import { CleanupItemRow } from "./CleanupItemRow";

export function SuggestionsStep({
  items,
  selectedIds,
  view,
  onViewChange,
  onSelectionChange,
  onNext,
}: {
  items: ScanItem[];
  selectedIds: string[];
  view: "risk" | "source";
  onViewChange: (view: "risk" | "source") => void;
  onSelectionChange: (ids: string[]) => void;
  onNext: () => void;
}) {
  const grouped = view === "risk" ? groupByRisk(items) : groupBySource(items);
  const totalCleanableBytes = estimateCleanableBytes(items);
  const selectedBytes = estimateSelectedBytes(selectedIds, items);
  const releasePercent =
    totalCleanableBytes > 0 ? Math.min(100, Math.round((selectedBytes / totalCleanableBytes) * 100)) : 0;

  return (
    <div className="step-content">
      <section className="suggestion-hero" aria-label="清理摘要">
        <div className="disk-illustration" aria-hidden="true">
          <HardDrive size={46} />
        </div>
        <div className="suggestion-summary">
          <p className="eyebrow">清理建议</p>
          <h2>
            <span>可释放 {formatBytes(totalCleanableBytes)}</span>
            <span>已选 {formatBytes(selectedBytes)}</span>
          </h2>
          <div className="release-track" aria-hidden="true">
            <div style={{ width: `${releasePercent}%` }} />
          </div>
          <p>推荐清理已自动勾选，高风险项目保持未选；你可以在下方按风险或来源快速调整。</p>
        </div>
        <button className="primary-button hero-action" onClick={onNext}>
          <Sparkles size={18} />
          一键清理已选项
        </button>
      </section>
      <div className="split-title">
        <div>
          <p className="eyebrow">项目明细</p>
          <h2>按低中高风险或软件来源查看</h2>
        </div>
        <div className="segmented" role="tablist">
          <button data-active={view === "risk"} onClick={() => onViewChange("risk")}>
            低 / 中 / 高
          </button>
          <button data-active={view === "source"} onClick={() => onViewChange("source")}>
            按来源
          </button>
        </div>
      </div>
      <div className="cleanup-list">
        {Object.entries(grouped).map(([group, groupItems]) => (
          <section className="cleanup-group" key={group}>
            <h3>{groupLabel(group)}</h3>
            {groupItems.length === 0 && <p className="empty-group">暂无项目</p>}
            {groupItems.map((item) => (
              <CleanupItemRow
                checked={selectedIds.includes(item.id)}
                item={item}
                key={item.id}
                onCheckedChange={(checked) =>
                  onSelectionChange(toggleSelection(selectedIds, item, checked))
                }
              />
            ))}
          </section>
        ))}
      </div>
      <button className="primary-button" onClick={onNext}>
        确认已选项目
      </button>
    </div>
  );
}

function groupLabel(group: string): string {
  const labels: Record<string, string> = {
    recommended: "推荐清理",
    optional: "可选清理",
    highRisk: "高风险清理",
    notCleanable: "不可清理",
    system: "系统清理",
    commonSoftware: "常用软件",
    wechat: "微信",
    qq: "QQ",
    workChat: "飞书 / 钉钉 / 企业微信",
    cloudDrive: "网盘与同步盘",
    installersOldVersions: "安装包与旧版本",
    otherLarge: "其他可疑大项",
  };
  return labels[group] ?? group;
}

function estimateCleanableBytes(items: ScanItem[]): number {
  return items.reduce((total, item) => {
    if (item.riskLevel === "notCleanable" || item.cleanupAction !== "directDelete") {
      return total;
    }
    return total + item.estimatedBytes;
  }, 0);
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(0)} MB`;
  return `${bytes} B`;
}
