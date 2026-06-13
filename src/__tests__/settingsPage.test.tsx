import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsPage } from "../components/settings/SettingsPage";
import * as v2Api from "../services/v2Api";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("SettingsPage", () => {
  it("loads default cleaner settings and disabled V0.2 OS integration entries", async () => {
    render(<SettingsPage />);

    expect(await screen.findByRole("checkbox", { name: /C:/ })).toBeChecked();
    expect(screen.getByRole("radio", { name: /500 MB/ })).toBeChecked();
    expect(screen.getByLabelText("历史保留天数")).toHaveValue(30);

    await waitFor(() => {
      expect(screen.getByRole("switch", { name: /桌面快捷方式/ })).not.toBeChecked();
    });
    expect(screen.getByRole("switch", { name: /桌面快捷方式/ })).toBeDisabled();
    expect(screen.getByRole("switch", { name: /C 盘右键菜单/ })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: /C 盘右键菜单/ })).toBeDisabled();
    expect(screen.getByRole("switch", { name: /定时扫描提醒/ })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: /定时扫描提醒/ })).toBeDisabled();
    expect(screen.getByText("V0.2 仅保留入口，不修改系统设置")).toBeInTheDocument();
  });

  it("shows a safe error state when settings fail to load", async () => {
    vi.spyOn(v2Api, "getDefaultCleanerSettings").mockRejectedValueOnce(new Error("C:\\Users\\Secret\\settings.json"));

    render(<SettingsPage />);

    expect(await screen.findByText("设置加载失败，请稍后重试")).toBeInTheDocument();
    expect(screen.queryByText(/C:\\Users\\Secret/)).not.toBeInTheDocument();
  });

  it("keeps current settings and shows a safe error when saving fails", async () => {
    vi.spyOn(v2Api, "saveCleanerSettings").mockRejectedValueOnce(new Error("D:\\private\\settings.json"));

    render(<SettingsPage />);

    expect(await screen.findByRole("checkbox", { name: /C:/ })).toBeChecked();

    fireEvent.click(screen.getByRole("button", { name: /保存设置/ }));

    expect(await screen.findByText("设置保存失败，本次未修改设置")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /C:/ })).toBeChecked();
    expect(screen.queryByText(/D:\\private/)).not.toBeInTheDocument();
  });
});
