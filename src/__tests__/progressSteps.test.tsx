import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CleanStep } from "../components/CleanStep";
import { ScanStep } from "../components/ScanStep";

describe("progress steps", () => {
  it("shows scan progress as a percentage", () => {
    render(<ScanStep progress={66} />);

    expect(screen.getByText("66%")).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("66");
  });

  it("shows cleanup progress as a percentage", () => {
    render(<CleanStep progress={42} />);

    expect(screen.getByText("42%")).toBeTruthy();
    expect(screen.getByRole("progressbar").getAttribute("aria-valuenow")).toBe("42");
  });
});
