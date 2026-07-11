import { useEffect, useRef, useState } from "react";
import { api, ChannelSpecInfo, ConnectionInfo, Reading } from "../ipc";
import Sparkline from "../Sparkline";

const POLL_INTERVAL_MS = 500;
const HISTORY_SAMPLES = 60; // ~30 s of context at the poll rate

export default function DashboardScreen({ connection }: { connection: ConnectionInfo }) {
  const [readings, setReadings] = useState<Reading[]>([]);
  const [specs, setSpecs] = useState<Record<string, ChannelSpecInfo>>({});
  const [error, setError] = useState<string | null>(null);
  // WCAG 2.2.2: auto-updating content needs a pause control.
  const [paused, setPaused] = useState(false);
  const history = useRef<Record<string, number[]>>({});
  const polling = useRef(false);

  useEffect(() => {
    api
      .listDefinitions()
      .then((defs) => {
        const def = defs.find((d) => d.id === connection.definition_id);
        const map: Record<string, ChannelSpecInfo> = {};
        for (const c of def?.channels ?? []) {
          if (c.spec) map[c.key] = c.spec;
        }
        setSpecs(map);
      })
      .catch(() => {});
  }, [connection.definition_id]);

  useEffect(() => {
    if (paused) return;
    let cancelled = false;
    const tick = async () => {
      if (polling.current) return; // don't overlap slow polls
      polling.current = true;
      try {
        const data = await api.pollLiveData();
        if (!cancelled) {
          for (const r of data) {
            const buf = (history.current[r.key] ??= []);
            buf.push(r.value);
            if (buf.length > HISTORY_SAMPLES) buf.shift();
          }
          setReadings(data);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        polling.current = false;
      }
    };
    tick();
    const id = setInterval(tick, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [paused]);

  return (
    <div className="panel panel-wide">
      <div className="panel-head">
        <h2>Live data</h2>
        <button
          className="btn btn-small"
          onClick={() => setPaused((p) => !p)}
          aria-pressed={paused}
        >
          {paused ? "Resume updates" : "Pause updates"}
        </button>
      </div>
      <p className="muted small">
        Polling every {POLL_INTERVAL_MS} ms — sparklines show the last ~
        {Math.round((HISTORY_SAMPLES * POLL_INTERVAL_MS) / 1000)} s. Hover or focus a trend
        and use the arrow keys for exact samples.
      </p>
      {paused && (
        <p className="muted small" role="status">
          Updates paused — values show the last reading before pausing.
        </p>
      )}
      {error && (
        <div className="error-box" role="alert">
          {error}
        </div>
      )}

      <div className="gauges">
        {readings.map((r) => {
          const spec = specs[r.key] ?? null;
          const hasBounds = spec !== null && (spec.min !== null || spec.max !== null);
          const inRange = hasBounds
            ? (spec!.min === null || r.value >= spec!.min) &&
              (spec!.max === null || r.value <= spec!.max)
            : null;
          return (
            <div className="gauge" key={r.key}>
              <div className="gauge-name">{r.name}</div>
              <div className="gauge-value">
                {formatValue(r.value)}
                <span className="gauge-unit">{r.unit}</span>
              </div>
              {inRange !== null && (
                <div
                  className={`gauge-status ${inRange ? "gauge-status-ok" : "gauge-status-bad"}`}
                  title={spec!.condition}
                >
                  <span aria-hidden="true">{inRange ? "✓" : r.value > (spec!.max ?? Infinity) ? "▲" : "▼"}</span>
                  {inRange
                    ? `in range (${rangeText(spec!)})`
                    : `outside ${rangeText(spec!)}`}
                </div>
              )}
              {/* Fresh copy per render: the ring buffer mutates in place, so
                  handing the same array reference down would defeat the
                  sparkline's memoization and freeze it at its first frame. */}
              <Sparkline
                values={(history.current[r.key] ?? []).slice()}
                specMin={spec?.min ?? null}
                specMax={spec?.max ?? null}
                unit={r.unit}
              />
            </div>
          );
        })}
      </div>

      {readings.length === 0 && !error && (
        <div className="empty-state">
          <span className="glyph" aria-hidden="true">⏱</span>
          Waiting for the first live-data frame…
        </div>
      )}
    </div>
  );
}

function rangeText(spec: ChannelSpecInfo): string {
  if (spec.min !== null && spec.max !== null) return `${formatValue(spec.min)}–${formatValue(spec.max)}`;
  if (spec.min !== null) return `≥ ${formatValue(spec.min)}`;
  if (spec.max !== null) return `≤ ${formatValue(spec.max)}`;
  return spec.condition;
}

function formatValue(v: number): string {
  if (Math.abs(v) >= 100) return v.toFixed(0);
  if (Math.abs(v) >= 10) return v.toFixed(1);
  return v.toFixed(2);
}
