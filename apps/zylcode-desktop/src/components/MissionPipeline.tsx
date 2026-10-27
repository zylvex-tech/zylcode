/**
 * The home surface's lifecycle strip: intent → plan → evidence → verified.
 *
 * Purely decorative. The four labels carry the meaning on their own, so the
 * strip is readable with no motion at all — the travelling pulse only hints at
 * the order things happen in. All animation lives in CSS and is switched off
 * under `prefers-reduced-motion`, which keeps it out of the way for anyone who
 * asked for less movement.
 */
export const MISSION_STAGES = ["Intent", "Plan", "Evidence", "Verified"] as const;

/** Seconds for one full traversal. Exported so tests assert the real value. */
export const MISSION_FLOW_PERIOD_S = 4.6;

export const MissionPipeline: React.FC = () => {
  const lastIndex = MISSION_STAGES.length - 1;
  return (
    <div
      className="mission-flow"
      role="img"
      aria-label="Mission lifecycle: intent, plan, evidence, verified"
    >
      <div className="mission-flow__track" aria-hidden="true">
        <span className="mission-flow__pulse" />
      </div>
      <ol className="mission-flow__stages">
        {MISSION_STAGES.map((stage, i) => (
          <li
            key={stage}
            className="mission-flow__stage"
            style={
              {
                // The node lights when the pulse arrives, so the stagger is
                // proportional to the stage's position along the track.
                "--stage-delay": `${((i / lastIndex) * (MISSION_FLOW_PERIOD_S - 0.2)).toFixed(2)}s`,
              } as React.CSSProperties
            }
          >
            <span className="mission-flow__node" aria-hidden="true" />
            <span className="mission-flow__label">{stage}</span>
          </li>
        ))}
      </ol>
    </div>
  );
};

export default MissionPipeline;
