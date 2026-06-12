import "@testing-library/jest-dom/vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SettingsPage } from "../components/settings/SettingsPage";

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
});
