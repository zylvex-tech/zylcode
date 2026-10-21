import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ActivityRail } from "./ActivityRail";

describe("ActivityRail", () => {
  // Regression guard: the `intel` surface (repository-intelligence grid plus
  // the deterministic code-navigation views) existed with no control that
  // could ever activate it. The rail entry is what makes it product-reachable.
  it("offers Code Navigation and routes it to the intel activity", () => {
    const onActivityChange = vi.fn();

    render(<ActivityRail activeActivity="explorer" onActivityChange={onActivityChange} />);

    fireEvent.click(screen.getByLabelText("Code Navigation"));
    expect(onActivityChange).toHaveBeenCalledWith("intel");
  });

  it("shows its label on hover so the icon is not unexplained", () => {
    render(<ActivityRail activeActivity="explorer" onActivityChange={vi.fn()} />);

    fireEvent.mouseEnter(screen.getByLabelText("Code Navigation"));
    // The button itself carries only an icon; the label lives in the tooltip.
    expect(screen.getByText("Code Navigation")).toBeInTheDocument();
  });
});
