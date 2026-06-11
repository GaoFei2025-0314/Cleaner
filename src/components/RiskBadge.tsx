import type { RiskLevel } from "../domain/models";

const labels: Record<RiskLevel, string> = {
  recommended: "推荐",
  optional: "可选",
  highRisk: "高风险",
  notCleanable: "不可清理",
};

export function RiskBadge({ risk }: { risk: RiskLevel }) {
  return (
    <span className="risk-badge" data-risk={risk}>
      {labels[risk]}
    </span>
  );
}
