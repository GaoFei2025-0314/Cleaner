import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SuggestionsStep } from "../components/SuggestionsStep";
import type { ScanItem } from "../domain/models";

const items: ScanItem[] = [
  {
    id: "windows-temp",
    title: "Windows 临时文件",
    description: "系统临时文件",
    sourceCategory: "system",
    riskLevel: "recommended",
    cleanupAction: "directDelete",
    estimatedBytes: 48,
    defaultSelected: true,
    userVisiblePathHint: "Windows 临时目录",
    reasons: ["规则命中：Windows 临时文件"],
    warnings: [],
  },
];

describe("SuggestionsStep", () => {
  it("shows recommended system cleanup in the regular recommended section", () => {
    render(
      <SuggestionsStep
        items={items}
        selectedIds={["windows-temp"]}
        view="risk"
        onViewChange={vi.fn()}
        onSelectionChange={vi.fn()}
        onNext={vi.fn()}
      />,
    );

    expect(screen.getByText("可释放 48 B")).toBeTruthy();
    expect(screen.getByText("已选 48 B")).toBeTruthy();
    expect(screen.getByText("推荐清理")).toBeTruthy();
    expect(screen.getByText("Windows 临时文件")).toBeTruthy();
    expect((screen.getByRole("checkbox") as HTMLInputElement).disabled).toBe(false);
  });
});
