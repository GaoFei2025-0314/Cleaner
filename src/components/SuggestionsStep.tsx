import type { ScanItem } from "../domain/models";
import { groupByRisk, groupBySource } from "../domain/grouping";
import { toggleSelection } from "../domain/selection";
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
  return (
    <div className="step-content">
      <div className="split-title">
        <div>
          <p className="eyebrow">清理建议</p>
          <h2>按风险或来源查看同一批扫描结果</h2>
        </div>
        <div className="segmented" role="tablist">
          <button data-active={view === "risk"} onClick={() => onViewChange("risk")}>
            按风险
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
