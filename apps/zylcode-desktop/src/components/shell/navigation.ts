// ---------------------------------------------------------------------------
// Activity → surface routing
//
// The one place an activity becomes a surface, kept out of App.tsx so it can be
// inspected (and tested) without mounting the whole shell. A comment-only
// re-export would not do: the mapping is data, not part of a component module.
// ---------------------------------------------------------------------------

import type { ActivityId } from "./ActivityRail";
import type { SurfaceType } from "./SurfaceHost";

export const ACTIVITY_TO_SURFACE: Record<ActivityId, SurfaceType> = {
  explorer: "code",
  search: "search",
  "source-control": "source-control",
  intel: "intel",
  design: "design",
  missions: "missions",
  run: "runtime",
  evidence: "evidence-live",
  forge: "forge",
  extensions: "extensions",
  settings: "overview",
  account: "overview",
};
