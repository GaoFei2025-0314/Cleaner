import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HistoryPage } from "../components/history/HistoryPage";
import * as v2Api from "../services/v2Api";

afterEach(() => {
  vi.restoreAllMocks();
});

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

  it("shows a safe error state when history fails to load", async () => {
    vi.spyOn(v2Api, "listOperationHistory").mockRejectedValueOnce(new Error("C:\\Users\\Secret\\history.json"));

    render(<HistoryPage />);

    expect(await screen.findByText("清理历史加载失败，历史记录可能包含未脱敏内容")).toBeInTheDocument();
    expect(screen.queryByText(/C:\\Users\\Secret/)).not.toBeInTheDocument();
  });

  it("shows a safe error when clearing history fails", async () => {
    vi.spyOn(v2Api, "clearOperationHistory").mockRejectedValueOnce(new Error("C:\\Users\\Secret\\history.json"));

    render(<HistoryPage />);

    expect(await screen.findByText("重复文件清理")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /清空历史/ }));

    expect(await screen.findByText("清空历史失败")).toBeInTheDocument();
    expect(screen.queryByText(/C:\\Users\\Secret/)).not.toBeInTheDocument();
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
