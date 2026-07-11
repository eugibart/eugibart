import { useMemo, useState } from "react";

/**
 * Single-series sparkline for a stat tile.
 *
 * Data-viz rules applied (see the repo's design notes): one validated hue
 * (never the status colors — those are reserved for state), a 2px line, a
 * faint spec band showing the normal range when one exists, recessive
 * hairline bounds, and a hover layer (nearest-sample tooltip + marker dot).
 * The value text in the tile wears ink, not the series color.
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

  if (!geom) return <div className="spark" style={{ height: 44 }} />;

  const onMove = (e: React.MouseEvent<SVGSVGElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const frac = (e.clientX - rect.left) / rect.width;
    const i = Math.round(frac * (values.length - 1));
    setHover(Math.max(0, Math.min(values.length - 1, i)));
  };

  const fmt = (v: number) => (Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2));

  return (
    <div className="spark">
      {hover !== null && (
        <div className="spark-tip" style={{ left: `${(geom.x(hover) / W) * 100}%` }}>
          {fmt(values[hover])} {unit}
        </div>
      )}
      <svg
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        role="img"
        aria-label={`Recent trend, ${values.length} samples`}
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
