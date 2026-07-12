// Shared numeric display rules so every screen (gauges, compare table,
// health report inputs) rounds the same way: magnitude-based precision,
// not a blanket toFixed(2) that turns "1246 rpm" into "1246.00 rpm".

export function formatValue(v: number): string {
  if (Math.abs(v) >= 100) return v.toFixed(0);
  if (Math.abs(v) >= 10) return v.toFixed(1);
  return v.toFixed(2);
}

/**
 * Delta formatting for before/after tables: a change smaller than the
 * value's own display precision is noise, not signal — render it as "—"
 * instead of "+0.00" so the one delta that matters stands out.
 */
export function formatDelta(v: number | null): string {
  if (v === null) return "—";
  const magnitude = formatValue(Math.abs(v));
  if (Number.parseFloat(magnitude) === 0) return "—";
  return `${v > 0 ? "+" : "−"}${magnitude}`;
}
