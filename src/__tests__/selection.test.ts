import { describe, expect, it } from "vitest";
import type { ScanItem } from "../domain/models";
import { buildDefaultSelection, requiresHighRiskConfirmation, toggleSelection } from "../domain/selection";

const items: ScanItem[] = [
  {
    id: "temp",
    title: "用户临时文件",
    description: "临时文件",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "directDelete",
    estimatedBytes: 100,
    defaultSelected: true,
    userVisiblePathHint: "用户临时目录",
    reasons: ["推荐清理"],
    warnings: [],
  },
  {
    id: "windows-temp",
    title: "Windows 临时文件",
    description: "需要管理员权限",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "requiresAdmin",
    estimatedBytes: 150,
    defaultSelected: true,
    userVisiblePathHint: "Windows 临时目录",
    reasons: ["V0.1 仅展示管理员清理能力"],
    warnings: ["当前版本不会执行提权清理"],
  },
  {
    id: "wechat-video",
    title: "微信视频",
    description: "聊天视频",
    sourceCategory: "wechat",
    riskLevel: "highRisk",
    cleanupAction: "directDelete",
    estimatedBytes: 200,
    defaultSelected: false,
    userVisiblePathHint: "微信数据目录",
    reasons: ["用户手动确认后可清理"],
    warnings: ["可能删除聊天视频"],
  },
  {
    id: "config-ref",
    title: "工具运行目录",
    description: "被配置引用",
    sourceCategory: "installersOldVersions",
    riskLevel: "notCleanable",
    cleanupAction: "blockedByConfigReference",
    estimatedBytes: 300,
    defaultSelected: false,
    userVisiblePathHint: "工具目录",
    reasons: ["被配置引用"],
    warnings: [],
  },
];

describe("selection", () => {
  it("selects only default selected cleanable items", () => {
    expect(buildDefaultSelection(items)).toEqual(["temp"]);
  });

  it("does not select not cleanable items", () => {
    expect(toggleSelection(["temp"], items[3], true)).toEqual(["temp"]);
  });

  it("does not default select or toggle admin-required items in V0.1", () => {
    expect(buildDefaultSelection(items)).toEqual(["temp"]);
    expect(toggleSelection(["temp"], items[1], true)).toEqual(["temp"]);
  });

  it("allows users to select high risk cleanable items", () => {
    expect(toggleSelection(["temp"], items[2], true)).toEqual(["temp", "wechat-video"]);
  });

  it("requires confirmation when a high risk item is selected", () => {
    expect(requiresHighRiskConfirmation(["temp", "wechat-video"], items)).toBe(true);
  });
});
