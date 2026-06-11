import type { RiskLevel, ScanItem, SourceCategory } from "./models";

export type RiskGroups = Record<RiskLevel, ScanItem[]>;
export type SourceGroups = Record<SourceCategory, ScanItem[]>;

export function groupByRisk(items: ScanItem[]): RiskGroups {
  return {
    recommended: items.filter((item) => item.riskLevel === "recommended"),
    optional: items.filter((item) => item.riskLevel === "optional"),
    highRisk: items.filter((item) => item.riskLevel === "highRisk"),
    notCleanable: items.filter((item) => item.riskLevel === "notCleanable"),
  };
}

export function groupBySource(items: ScanItem[]): SourceGroups {
  return {
    system: items.filter((item) => item.sourceCategory === "system"),
    commonSoftware: items.filter((item) => item.sourceCategory === "commonSoftware"),
    wechat: items.filter((item) => item.sourceCategory === "wechat"),
    qq: items.filter((item) => item.sourceCategory === "qq"),
    workChat: items.filter((item) => item.sourceCategory === "workChat"),
    cloudDrive: items.filter((item) => item.sourceCategory === "cloudDrive"),
    installersOldVersions: items.filter((item) => item.sourceCategory === "installersOldVersions"),
    otherLarge: items.filter((item) => item.sourceCategory === "otherLarge"),
  };
}
