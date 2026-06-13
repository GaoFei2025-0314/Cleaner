export type OperationStatus = "running" | "completed" | "cancelled" | "failed";

export type OperationModule =
  | "cDriveCleanup"
  | "duplicateScan"
  | "duplicateCleanup"
  | "largeFileScan"
  | "largeFileMigration";

export type OriginalFilePolicy = "keepOriginal" | "moveOriginalToRecycleBin";

export type DuplicateFileType = "image" | "document" | "audio" | "video" | "archive" | "custom";

export type DuplicateGroupKind = "strict" | "suspected";

export type DuplicateRecommendedAction = "keep" | "clean" | "manualReview";

export type DuplicateDefaultStrategy = "cDriveFirstKeepNewest" | "keepNewest" | "keepOldest";

export type LargeFileCategory = "video" | "archive" | "installer" | "diskImage" | "document" | "other";

export type MigrationItemStatus = "copied" | "copiedAndFreed" | "skipped" | "failed";

export interface OperationStart {
  operationId: string;
}

export interface OperationProgressPayload {
  operationId: string;
  module: OperationModule;
  stage: string;
  percent: number;
  currentLocationHint: string;
  currentFileType: string | null;
  scannedFiles: number;
  foundGroups: number;
  foundItems: number;
  foundBytes: number;
  processedItems: number;
  successCount: number;
  skippedCount: number;
  failedCount: number;
}

export interface OperationFinishedPayload<T = unknown> {
  operationId: string;
  module: OperationModule;
  status: OperationStatus;
  result: T | null;
  message: string | null;
}

export interface CleanerSettings {
  protectedPaths: string[];
  defaultScanDrives: string[];
  duplicateDefaultStrategy: DuplicateDefaultStrategy;
  largeFileDefaultThresholdBytes: number;
  historyRetentionDays: number;
  desktopShortcutEnabled: boolean;
  cDriveContextMenuEnabled: boolean;
  scheduledScanReminderEnabled: boolean;
}

export interface HistoryEntry {
  historyId: string;
  module: OperationModule;
  startedAt: string;
  finishedAt: string;
  totalBytes: number;
  freedBytes: number;
  cDriveFreedBytes: number;
  otherDriveFreedBytes: number;
  successCount: number;
  skippedCount: number;
  failedCount: number;
  errorCategories: string[];
}

export interface DuplicateScanRequest {
  selectedDrives: string[];
  customFolders: string[];
  fileTypes: DuplicateFileType[];
  customExtensions: string[];
  includeSuspected: boolean;
  minSizeBytes: number;
  protectedPaths: string[];
}

export interface DuplicateScanReport {
  strictGroups: DuplicateFileGroup[];
  suspectedGroups: DuplicateFileGroup[];
  scannedFiles: number;
  skippedLocations: number;
  totalReclaimableBytes: number;
  cDriveReclaimableBytes: number;
  otherDriveReclaimableBytes: number;
}

export interface DuplicateFileGroup {
  groupId: string;
  strictDuplicate: boolean;
  totalBytes: number;
  reclaimableBytes: number;
  files: DuplicateFileEntry[];
  recommendedSelectionReason: string;
}

export interface DuplicateFileEntry {
  entryId: string;
  displayName: string;
  drive: string;
  visibleLocationHint: string;
  sizeBytes: number;
  modifiedAt: string;
  hashFingerprintId: string;
  selected: boolean;
  protected: boolean;
  recommendedAction: DuplicateRecommendedAction;
}

export interface DuplicateCleanupRequest {
  groups: DuplicateCleanupGroupRequest[];
  protectedOverrideConfirmed: boolean;
}

export interface DuplicateCleanupGroupRequest {
  groupId: string;
  files: DuplicateCleanupFileRequest[];
}

export interface DuplicateCleanupFileRequest {
  entryId: string;
  selected: boolean;
  protected: boolean;
}

export interface DuplicateCleanupReport {
  processedFiles: number;
  successCount: number;
  skippedCount: number;
  failedCount: number;
  freedBytes: number;
  cDriveFreedBytes: number;
  otherDriveFreedBytes: number;
}

export interface LargeFileScanRequest {
  selectedDrives: string[];
  customFolders: string[];
  minSizeBytes: number;
  protectedPaths: string[];
  skipSystemDirs: boolean;
  skipProgramDirs: boolean;
}

export interface LargeFileScanReport {
  items: LargeFileItem[];
  scannedFiles: number;
  skippedLocations: number;
  totalBytes: number;
  cDriveBytes: number;
  otherDriveBytes: number;
}

export interface LargeFileItem {
  itemId: string;
  displayName: string;
  drive: string;
  visibleLocationHint: string;
  sizeBytes: number;
  modifiedAt: string;
  category: LargeFileCategory;
  selected: boolean;
  protected: boolean;
  recommended: boolean;
}

export interface MigrationRequest {
  selectedItemIds: string[];
  scanReport: LargeFileScanReport;
  targetFolder: string;
  originalFilePolicy: OriginalFilePolicy;
  protectedOverrideConfirmed: boolean;
}

export interface MigrationResult {
  copiedCount: number;
  movedToRecycleBinCount: number;
  skippedCount: number;
  failedCount: number;
  totalCopiedBytes: number;
  totalFreedBytes: number;
  cDriveFreedBytes: number;
  itemResults: MigrationItemResult[];
}

export interface MigrationItemResult {
  itemId: string;
  status: MigrationItemStatus;
  category: LargeFileCategory;
  bytesCopied: number;
  bytesFreed: number;
  message: string;
}
