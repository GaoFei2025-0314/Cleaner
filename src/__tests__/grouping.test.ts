import { describe, expect, it } from "vitest";
import type { ScanItem } from "../domain/models";
import { groupByRisk, groupBySource } from "../domain/grouping";

const items: ScanItem[] = [
  {
    id: "a",
    title: "系统临时文件",
    description: "临时文件",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "directDelete",
    estimatedBytes: 1,
    defaultSelected: true,
    userVisiblePathHint: "系统",
    reasons: [],
    warnings: [],
  },
  {
    id: "b",
    title: "微信图片",
    description: "图片",
    sourceCategory: "wechat",
    riskLevel: "highRisk",
    cleanupAction: "directDelete",
    estimatedBytes: 2,
    defaultSelected: false,
    userVisiblePathHint: "微信",
    reasons: [],
    warnings: [],
  },
];

describe("grouping", () => {
  it("groups items by risk", () => {
    const grouped = groupByRisk(items);
    expect(grouped.recommended.map((item) => item.id)).toEqual(["a"]);
    expect(grouped.highRisk.map((item) => item.id)).toEqual(["b"]);
  });

  it("groups items by source", () => {
    const grouped = groupBySource(items);
    expect(grouped.system.map((item) => item.id)).toEqual(["a"]);
    expect(grouped.wechat.map((item) => item.id)).toEqual(["b"]);
  });
});
