export type RiskLevel = "recommended" | "optional" | "highRisk" | "notCleanable";

export type SourceCategory =
  | "system"
  | "commonSoftware"
  | "wechat"
  | "qq"
  | "workChat"
  | "cloudDrive"
  | "installersOldVersions"
  | "otherLarge";

export type CleanupAction =
  | "directDelete"
  | "requiresAdmin"
  | "explainOnly"
  | "blockedByProcess"
  | "blockedByConfigReference";

export interface DriveSummary {
  drive: "C:";
  totalBytes: number;
  freeBytes: number;
}

export interface ScanItem {
  id: string;
  title: string;
  description: string;
  sourceCategory: SourceCategory;
  riskLevel: RiskLevel;
  cleanupAction: CleanupAction;
  estimatedBytes: number;
  defaultSelected: boolean;
  userVisiblePathHint: string;
  technicalPath?: string;
  reasons: string[];
  warnings: string[];
}

export interface ScanReport {
  driveSummary: DriveSummary;
  items: ScanItem[];
  partial: boolean;
  scanStartedAt: string;
  scanFinishedAt: string;
}

export interface CleanupSelection {
  selectedItemIds: string[];
  highRiskConfirmed: boolean;
  requestAdminMode: boolean;
}

export interface CleanupItemResult {
  itemId: string;
  status: "deleted" | "skipped" | "failed";
  freedBytes: number;
  message: string;
}

export interface CleanupResult {
  results: CleanupItemResult[];
  totalFreedBytes: number;
  finishedAt: string;
}
