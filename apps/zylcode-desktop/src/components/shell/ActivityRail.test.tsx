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

  it("shows the keyboard shortcut alongside the label it belongs to", () => {
    render(<ActivityRail activeActivity="explorer" onActivityChange={vi.fn()} />);

    fireEvent.mouseEnter(screen.getByLabelText("Missions"));

    expect(screen.getByText("Missions")).toBeInTheDocument();
    expect(screen.getByText("Ctrl+M")).toBeInTheDocument();
  });

  it("routes a bottom utility to its own handler, not the activity router", () => {
    const onActivityChange = vi.fn();
    const onBottomUtilityClick = vi.fn();

    render(
      <ActivityRail
        activeActivity="explorer"
        onActivityChange={onActivityChange}
        onBottomUtilityClick={onBottomUtilityClick}
      />,
    );

    fireEvent.click(screen.getByLabelText("Settings"));

    expect(onBottomUtilityClick).toHaveBeenCalledWith("settings");
    expect(onActivityChange).not.toHaveBeenCalled();
  });

  it("collapses the tooltip when the pointer leaves the button", () => {
    render(<ActivityRail activeActivity="explorer" onActivityChange={vi.fn()} />);
    const missions = screen.getByLabelText("Missions");

    fireEvent.mouseEnter(missions);
    expect(missions).toHaveAttribute("aria-expanded", "true");

    fireEvent.mouseLeave(missions);
    expect(missions).toHaveAttribute("aria-expanded", "false");
  });

  it("collapses the tooltip when the pointer leaves the rail", () => {
    const { container } = render(
      <ActivityRail activeActivity="explorer" onActivityChange={vi.fn()} />,
    );
    const missions = screen.getByLabelText("Missions");
    const nav = container.querySelector("nav") as HTMLElement;

    fireEvent.mouseEnter(missions);
    expect(missions).toHaveAttribute("aria-expanded", "true");

    fireEvent.mouseLeave(nav);
    expect(missions).toHaveAttribute("aria-expanded", "false");
  });

  it("collapses the tooltip when the press lands outside the rail", () => {
    render(<ActivityRail activeActivity="explorer" onActivityChange={vi.fn()} />);
    const missions = screen.getByLabelText("Missions");

    fireEvent.mouseEnter(missions);
    expect(missions).toHaveAttribute("aria-expanded", "true");

    fireEvent.mouseDown(document.body);
    expect(missions).toHaveAttribute("aria-expanded", "false");
  });
});
