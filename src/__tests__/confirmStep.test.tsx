import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ConfirmStep } from "../components/ConfirmStep";
import type { ScanItem } from "../domain/models";

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
    reasons: [],
    warnings: [],
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
    userVisiblePathHint: "微信视频缓存",
    reasons: [],
    warnings: [],
  },
  {
    id: "wechat-data-root",
    title: "微信数据根目录",
    description: "微信完整数据目录",
    sourceCategory: "wechat",
    riskLevel: "notCleanable",
    cleanupAction: "explainOnly",
    estimatedBytes: 300,
    defaultSelected: false,
    userVisiblePathHint: "微信用户数据根目录",
    reasons: [],
    warnings: [],
  },
];

describe("ConfirmStep", () => {
  it("summarizes untouched items and lists selected high risk items separately", () => {
    render(
      <ConfirmStep
        items={items}
        selectedIds={["temp", "wechat-video"]}
        highRiskConfirmed={false}
        onHighRiskConfirmed={vi.fn()}
        onBack={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    expect(screen.getByText("不会触碰 1 项")).toBeTruthy();
    expect(screen.getByText("高风险项目")).toBeTruthy();
    expect(screen.getAllByText("微信视频")).toHaveLength(2);
  });
});
