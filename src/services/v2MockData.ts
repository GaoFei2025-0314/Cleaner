import type {
  CleanerSettings,
  DuplicateCleanupReport,
  DuplicateScanReport,
  HistoryEntry,
  LargeFileScanReport,
  MigrationResult,
} from "../domain/v2";

export const mockCleanerSettings: CleanerSettings = {
  protectedPaths: ["C:\\Windows", "C:\\Program Files", "C:\\Program Files (x86)"],
  defaultScanDrives: ["C:"],
  duplicateDefaultStrategy: "cDriveFirstKeepNewest",
  largeFileDefaultThresholdBytes: 500 * 1024 * 1024,
  historyRetentionDays: 30,
  desktopShortcutEnabled: false,
  cDriveContextMenuEnabled: false,
  scheduledScanReminderEnabled: false,
};

export const mockDuplicateScanReport: DuplicateScanReport = {
  strictGroups: [
    {
      groupId: "dup-strict-001",
      strictDuplicate: true,
      totalBytes: 734_003_200,
      reclaimableBytes: 367_001_600,
      recommendedSelectionReason: "保留 C 盘较新的副本，建议清理其他位置的重复文件。",
      files: [
        {
          entryId: "dup-strict-001-a",
          displayName: "project-export-copy.pdf",
          drive: "C:",
          visibleLocationHint: "C:\\...\\Documents",
          sizeBytes: 367_001_600,
          modifiedAt: "2026-05-28T08:30:00.000Z",
          hashFingerprintId: "mock-fingerprint-a",
          selected: false,
          protected: false,
          recommendedAction: "keep",
        },
        {
          entryId: "dup-strict-001-b",
          displayName: "project-export-copy.pdf",
          drive: "D:",
          visibleLocationHint: "D:\\...\\Archive",
          sizeBytes: 367_001_600,
          modifiedAt: "2026-05-21T11:15:00.000Z",
          hashFingerprintId: "mock-fingerprint-a",
          selected: true,
          protected: false,
          recommendedAction: "clean",
        },
      ],
    },
  ],
  suspectedGroups: [
    {
      groupId: "dup-suspected-001",
      strictDuplicate: false,
      totalBytes: 251_658_240,
      reclaimableBytes: 0,
      recommendedSelectionReason: "文件名相近但内容未严格匹配，建议手动确认。",
      files: [
        {
          entryId: "dup-suspected-001-a",
          displayName: "media-cache-preview.mp4",
          drive: "C:",
          visibleLocationHint: "C:\\...\\Temp",
          sizeBytes: 125_829_120,
          modifiedAt: "2026-05-20T10:00:00.000Z",
          hashFingerprintId: "mock-suspected-a",
          selected: false,
          protected: false,
          recommendedAction: "manualReview",
        },
        {
          entryId: "dup-suspected-001-b",
          displayName: "media-cache-preview (copy).mp4",
          drive: "C:",
          visibleLocationHint: "C:\\...\\Downloads",
          sizeBytes: 125_829_120,
          modifiedAt: "2026-05-19T09:45:00.000Z",
          hashFingerprintId: "mock-suspected-b",
          selected: false,
          protected: false,
          recommendedAction: "manualReview",
        },
      ],
    },
  ],
  scannedFiles: 42_000,
  skippedLocations: 3,
  totalReclaimableBytes: 367_001_600,
  cDriveReclaimableBytes: 0,
  otherDriveReclaimableBytes: 367_001_600,
};

export const mockLargeFileScanReport: LargeFileScanReport = {
  items: [
    {
      itemId: "large-c-001",
      displayName: "large-video-file.mp4",
      drive: "C:",
      visibleLocationHint: "C:\\...\\Videos",
      sizeBytes: 4_294_967_296,
      modifiedAt: "2026-05-12T14:00:00.000Z",
      category: "video",
      selected: true,
      protected: false,
      recommended: true,
    },
    {
      itemId: "large-c-002",
      displayName: "installer-package.iso",
      drive: "C:",
      visibleLocationHint: "C:\\...\\Downloads",
      sizeBytes: 2_147_483_648,
      modifiedAt: "2026-04-02T12:00:00.000Z",
      category: "diskImage",
      selected: true,
      protected: false,
      recommended: true,
    },
    {
      itemId: "large-protected-001",
      displayName: "protected-archive.zip",
      drive: "C:",
      visibleLocationHint: "C:\\...\\ProgramData",
      sizeBytes: 805_306_368,
      modifiedAt: "2026-03-15T09:00:00.000Z",
      category: "archive",
      selected: false,
      protected: true,
      recommended: false,
    },
  ],
  scannedFiles: 42_000,
  skippedLocations: 5,
  totalBytes: 7_247_757_312,
  cDriveBytes: 7_247_757_312,
  otherDriveBytes: 0,
};

export const mockDuplicateCleanupReport: DuplicateCleanupReport = {
  processedFiles: 1,
  successCount: 1,
  skippedCount: 0,
  failedCount: 0,
  freedBytes: 367_001_600,
  cDriveFreedBytes: 0,
  otherDriveFreedBytes: 367_001_600,
};

export const mockMigrationResult: MigrationResult = {
  copiedCount: 2,
  movedToRecycleBinCount: 2,
  skippedCount: 1,
  failedCount: 0,
  totalCopiedBytes: 6_442_450_944,
  totalFreedBytes: 6_442_450_944,
  cDriveFreedBytes: 6_442_450_944,
  itemResults: [
    {
      itemId: "large-c-001",
      status: "copiedAndFreed",
      category: "video",
      bytesCopied: 4_294_967_296,
      bytesFreed: 4_294_967_296,
      message: "已复制到目标位置，并将原文件移入回收站。",
    },
    {
      itemId: "large-c-002",
      status: "copiedAndFreed",
      category: "diskImage",
      bytesCopied: 2_147_483_648,
      bytesFreed: 2_147_483_648,
      message: "已复制到目标位置，并将原文件移入回收站。",
    },
    {
      itemId: "large-protected-001",
      status: "skipped",
      category: "archive",
      bytesCopied: 0,
      bytesFreed: 0,
      message: "受保护位置默认跳过。",
    },
  ],
};

export const mockHistory: HistoryEntry[] = [
  {
    historyId: "history-duplicate-001",
    module: "duplicateCleanup",
    startedAt: "2026-06-01T08:00:00.000Z",
    finishedAt: "2026-06-01T08:03:00.000Z",
    totalBytes: 734_003_200,
    freedBytes: 367_001_600,
    cDriveFreedBytes: 0,
    otherDriveFreedBytes: 367_001_600,
    successCount: 1,
    skippedCount: 0,
    failedCount: 0,
    errorCategories: [],
  },
  {
    historyId: "history-migration-001",
    module: "largeFileMigration",
    startedAt: "2026-06-02T09:00:00.000Z",
    finishedAt: "2026-06-02T09:08:00.000Z",
    totalBytes: 7_247_757_312,
    freedBytes: 6_442_450_944,
    cDriveFreedBytes: 6_442_450_944,
    otherDriveFreedBytes: 0,
    successCount: 2,
    skippedCount: 1,
    failedCount: 0,
    errorCategories: [],
  },
];

export function cloneMock<T>(value: T): T {
  return structuredClone(value);
}
