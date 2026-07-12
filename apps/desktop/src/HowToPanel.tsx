import { useState } from "react";
import { AccessGuideInfo, BodyStyle } from "./ipc";
import BikeDiagram, { ZONE_LABELS } from "./BikeDiagram";
import SourceLink from "./SourceLink";

/**
 * A "how do I get to it?" guide: what to have ready, where the port is
 * (schematic diagram with an animated marker), and the access steps as an
 * interactive checklist — ticking a step moves the highlight to the next
 * one, so mid-job you always know where you are.
 *
 * guide === null renders the honest fallback: we don't know this bike's
 * location yet, and we say so instead of drawing a guess.
 */
export default function HowToPanel({
  guide,
  bodyStyle,
  fallback,
}: {
  guide: AccessGuideInfo | null;
  bodyStyle: BodyStyle;
  fallback: string;
}) {
  const [done, setDone] = useState<boolean[]>([]);

  if (!guide) {
    return (
      <div className="howto">
        <p className="muted">{fallback}</p>
        <p className="muted small">
          Once you've found it, the connection troubleshooter above will tell you which layer
          fails if something's miswired.
        </p>
      </div>
    );
  }

  const doneCount = guide.steps.filter((_, i) => done[i]).length;
  const nextIndex = guide.steps.findIndex((_, i) => !done[i]);
  const toggle = (i: number) =>
    setDone((d) => {
      const next = d.slice(0, guide.steps.length);
      while (next.length < guide.steps.length) next.push(false);
      next[i] = !next[i];
      return next;
    });

  return (
    <div className="howto">
      {guide.tools.length > 0 && (
        <>
          <h4 className="howto-subhead">What you need</h4>
          <ul className="howto-tools">
            {guide.tools.map((t, i) => (
              <li key={i}>{t}</li>
            ))}
          </ul>
        </>
      )}

      <h4 className="howto-subhead">Where it is</h4>
      <p className="howto-summary">{guide.summary}</p>
      <div className="howto-diagram">
        <BikeDiagram zone={guide.zone} bodyStyle={bodyStyle} />
        <p className="howto-caption">
          <span className="howto-caption-dot" aria-hidden="true" /> Approximate location:{" "}
          {ZONE_LABELS[guide.zone]}
        </p>
      </div>

      {guide.steps.length > 0 && (
        <>
          <h4 className="howto-subhead">Steps</h4>
          <ol className="howto-steps">
            {guide.steps.map((step, i) => (
              <li
                key={i}
                className={`howto-step ${done[i] ? "howto-step-done" : ""} ${
                  i === nextIndex ? "howto-step-next" : ""
                }`}
              >
                <label>
                  <input type="checkbox" checked={done[i] ?? false} onChange={() => toggle(i)} />
                  <span>{step}</span>
                </label>
              </li>
            ))}
          </ol>
          <p className="muted small" role="status">
            {doneCount} of {guide.steps.length} steps done
            {doneCount === guide.steps.length ? " — ready to connect." : ""}
          </p>
        </>
      )}

      {guide.verify_note && (
        <div className="help-callout">
          <span className="help-callout-label">Before you trust this</span>
          <p>{guide.verify_note}</p>
        </div>
      )}
      {guide.source && guide.source_url && (
        <SourceLink site={guide.source} url={guide.source_url} />
      )}
    </div>
  );
}
