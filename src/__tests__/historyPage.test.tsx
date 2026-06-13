import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { HistoryPage } from "../components/history/HistoryPage";

describe("HistoryPage", () => {
  it("renders sanitized operation history without full path-like strings", async () => {
    const { container } = render(<HistoryPage />);

    expect(await screen.findByText("重复文件清理")).toBeInTheDocument();
    expect(screen.getByText("大文件迁移")).toBeInTheDocument();
    expect(screen.getByText("成功")).toBeInTheDocument();
    expect(screen.getByText("跳过")).toBeInTheDocument();
    expect(screen.getByText("失败")).toBeInTheDocument();

    expect(container.textContent).not.toMatch(/[A-Z]:[\\/][^\s]+/);
    expect(container.textContent).not.toMatch(/\.[a-z0-9]{2,5}\b/i);
  });

  it("shows sanitized C drive cleanup history when the C drive tab is selected", async () => {
    const { container } = render(<HistoryPage />);

    await screen.findByText("重复文件清理");

    fireEvent.click(screen.getByRole("tab", { name: /C 盘清理/ }));

    expect(await screen.findByRole("cell", { name: "C 盘清理" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "860 MB" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "8" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "1" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "0" })).toBeInTheDocument();
    expect(container.textContent).not.toMatch(/[A-Z]:[\\/][^\s]+/);
    expect(container.textContent).not.toMatch(/\.[a-z0-9]{2,5}\b/i);
  });

  it("clears operation history", async () => {
    render(<HistoryPage />);

    expect(await screen.findByText("重复文件清理")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /清空历史/ }));

    await waitFor(() => {
      expect(screen.getByText("暂无清理历史")).toBeInTheDocument();
    });
  });
});
