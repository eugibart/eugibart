import { useEffect, useMemo, useState } from "react";

/**
 * Single-series sparkline for a stat tile.
 *
 * Data-viz rules applied (see the repo's design notes): one validated hue
 * (never the status colors — those are reserved for state), a 2px line, a
 * faint spec band showing the normal range when one exists, recessive
 * hairline bounds, and a hover layer (nearest-sample tooltip + marker dot).
 * The value text in the tile wears ink, not the series color.
 *
 * Accessibility: the tile is focusable; Left/Right/Home/End step through
 * samples (WCAG 2.1.1 — everything hover gives, keys give too), the inspected
 * value is echoed to a polite live region, and Escape dismisses the tooltip
 * whether it was opened by pointer or keyboard (WCAG 1.4.13).
 */
export default function Sparkline({
  values,
  specMin,
  specMax,
  unit,
}: {
  values: number[];
  specMin: number | null;
  specMax: number | null;
  unit: string;
}) {
  const [hover, setHover] = useState<number | null>(null);

  const W = 220;
  const H = 44;
  const PAD = 3;

  const geom = useMemo(() => {
    if (values.length < 2) return null;
    let lo = Math.min(...values);
    let hi = Math.max(...values);
    // Include the spec band in the scale so the band is always visible
    // context, then pad so the line never kisses the frame.
    if (specMin !== null) lo = Math.min(lo, specMin);
    if (specMax !== null) hi = Math.max(hi, specMax);
    if (hi - lo < 1e-9) {
      hi += 1;
      lo -= 1;
    }
    const span = hi - lo;
    lo -= span * 0.08;
    hi += span * 0.08;

    const x = (i: number) => PAD + (i / (values.length - 1)) * (W - 2 * PAD);
    const y = (v: number) => H - PAD - ((v - lo) / (hi - lo)) * (H - 2 * PAD);
    const path = values.map((v, i) => `${i === 0 ? "M" : "L"}${x(i).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
    return { x, y, path };
  }, [values, specMin, specMax]);

  // WCAG 1.4.13: the hover tooltip must be dismissable without moving the
  // pointer. Listen globally so Escape works for pointer-opened tooltips too
  // (the tile isn't focused while hovering).
  useEffect(() => {
    if (hover === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setHover(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [hover]);

  if (!geom) return <div className="spark" style={{ height: 44 }} />;

  const onMove = (e: React.MouseEvent<SVGSVGElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const frac = (e.clientX - rect.left) / rect.width;
    const i = Math.round(frac * (values.length - 1));
    setHover(Math.max(0, Math.min(values.length - 1, i)));
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const last = values.length - 1;
    let next: number | null | undefined;
    if (e.key === "ArrowLeft") next = Math.max(0, (hover ?? values.length) - 1);
    else if (e.key === "ArrowRight") next = Math.min(last, (hover ?? -1) + 1);
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = last;
    else return;
    e.preventDefault();
    setHover(next);
  };

  const fmt = (v: number) => (Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2));

  return (
    <div
      className="spark"
      tabIndex={0}
      role="img"
      aria-label={`Recent trend, ${values.length} samples, latest ${fmt(values[values.length - 1])} ${unit}. Press Left or Right arrow to inspect samples.`}
      onKeyDown={onKeyDown}
      onBlur={() => setHover(null)}
    >
      <span className="visually-hidden" aria-live="polite">
        {hover !== null ? `Sample ${hover + 1} of ${values.length}: ${fmt(values[hover])} ${unit}` : ""}
      </span>
      {hover !== null && (
        <div className="spark-tip" style={{ left: `${(geom.x(hover) / W) * 100}%` }}>
          {fmt(values[hover])} {unit}
        </div>
      )}
      <svg
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        aria-hidden="true"
        onMouseMove={onMove}
        onMouseLeave={() => setHover(null)}
      >
        {specMin !== null && specMax !== null && (
          <rect
            x={PAD}
            width={W - 2 * PAD}
            y={geom.y(specMax)}
            height={Math.max(0, geom.y(specMin) - geom.y(specMax))}
            fill="var(--viz-band)"
          />
        )}
        {specMin !== null && (
          <line x1={PAD} x2={W - PAD} y1={geom.y(specMin)} y2={geom.y(specMin)} stroke="var(--viz-grid)" strokeWidth="1" />
        )}
        {specMax !== null && (
          <line x1={PAD} x2={W - PAD} y1={geom.y(specMax)} y2={geom.y(specMax)} stroke="var(--viz-grid)" strokeWidth="1" />
        )}
        <path d={geom.path} fill="none" stroke="var(--viz-series)" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" vectorEffect="non-scaling-stroke" />
        {hover !== null && (
          <circle cx={geom.x(hover)} cy={geom.y(values[hover])} r="3.5" fill="var(--viz-series)" stroke="var(--bg-raised)" strokeWidth="2" />
        )}
      </svg>
    </div>
  );
}
