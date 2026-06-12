import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AppShell } from "../components/AppShell";

describe("AppShell", () => {
  it("shows the generated Cleaner bitmap logo", () => {
    render(
      <AppShell currentStep={0} report={null}>
        <div />
      </AppShell>,
    );

    const logo = screen.getByAltText("Cleaner logo") as HTMLImageElement;

    expect(logo.tagName).toBe("IMG");
    expect(logo.getAttribute("src")).toContain("cleaner-logo.png");
  });
});
