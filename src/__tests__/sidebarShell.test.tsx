import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { hasBlockingCDriveWork } from "../App";
import { SidebarShell, type CleanerModule } from "../components/SidebarShell";

describe("SidebarShell", () => {
  const modules = [
    "C 盘清理",
    "重复文件清理",
    "大文件迁移",
    "隐私清理",
    "清理历史",
    "设置",
  ];

  it("renders Cleaner navigation with the bitmap logo", () => {
    render(
      <SidebarShell activeModule="cDrive" onModuleChange={() => undefined}>
        <div>当前页面</div>
      </SidebarShell>,
    );

    const logo = screen.getByAltText("Cleaner logo") as HTMLImageElement;
    expect(logo.getAttribute("src")).toContain("cleaner-logo.png");

    for (const moduleName of modules) {
      expect(screen.getByRole("button", { name: new RegExp(moduleName) })).toBeInTheDocument();
    }
  });

  it("shows the V0.3 inline message when privacy cleanup is selected", () => {
    function Harness() {
      const [activeModule, setActiveModule] = useState<CleanerModule>("cDrive");
      return (
        <SidebarShell activeModule={activeModule} onModuleChange={setActiveModule}>
          <div>当前页面</div>
        </SidebarShell>
      );
    }

    render(<Harness />);

    fireEvent.click(screen.getByRole("button", { name: /隐私清理/ }));

    expect(screen.getByText("隐私清理将在 V0.3 提供")).toBeInTheDocument();
  });

  it("confirms before switching modules when work is blocking", () => {
    const onModuleChange = vi.fn();
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(
      <SidebarShell activeModule="cDrive" hasBlockingWork onModuleChange={onModuleChange}>
        <div>当前页面</div>
      </SidebarShell>,
    );

    fireEvent.click(screen.getByRole("button", { name: /设置/ }));

    expect(confirmSpy).toHaveBeenCalledWith("当前清理任务或选择尚未完成，确定要切换页面吗？");
    expect(onModuleChange).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
  });

  it("treats scanned C drive results as blocking even when all selections are empty", () => {
    expect(
      hasBlockingCDriveWork({
        activeModule: "cDrive",
        hasReport: true,
        step: "suggestions",
      }),
    ).toBe(true);
  });
});
