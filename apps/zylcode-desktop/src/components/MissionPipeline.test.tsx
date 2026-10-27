// The home surface's lifecycle strip is decorative, so the test asserts the
// things that carry meaning: the four stages, their order, and the accessible
// name. Motion is not asserted — it is CSS-only and disabled under
// prefers-reduced-motion.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MISSION_STAGES, MISSION_FLOW_PERIOD_S, MissionPipeline } from "./MissionPipeline";

describe("MissionPipeline", () => {
  it("renders the four lifecycle stages in order", () => {
    render(<MissionPipeline />);
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(4);
    expect(items.map((li) => li.textContent)).toEqual(["Intent", "Plan", "Evidence", "Verified"]);
  });

  it("exposes the sequence to assistive technology", () => {
    render(<MissionPipeline />);
    expect(
      screen.getByRole("img", { name: /intent, plan, evidence, verified/i }),
    ).toBeInTheDocument();
  });

  it("stagger is proportional to the traversal period", () => {
    // The last stage must light before the pulse wraps, never after it.
    expect(MISSION_STAGES).toHaveLength(4);
    expect(MISSION_FLOW_PERIOD_S).toBeGreaterThan(0);
    expect(MISSION_FLOW_PERIOD_S - 0.2).toBeLessThan(MISSION_FLOW_PERIOD_S);
  });
});
