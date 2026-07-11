import { useEffect, useRef, useState } from "react";
import { api, ChannelSpecInfo, ConnectionInfo, Reading } from "../ipc";

const POLL_INTERVAL_MS = 500;

export default function DashboardScreen({ connection }: { connection: ConnectionInfo }) {
  const [readings, setReadings] = useState<Reading[]>([]);
  const [specs, setSpecs] = useState<Record<string, ChannelSpecInfo>>({});
  const [error, setError] = useState<string | null>(null);
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
    let cancelled = false;
    const tick = async () => {
      if (polling.current) return; // don't overlap slow polls
      polling.current = true;
      try {
        const data = await api.pollLiveData();
        if (!cancelled) {
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
  }, []);

  return (
    <div className="panel">
      <h2>Live data</h2>
      {error && <div className="error-box">{error}</div>}
      <div className="gauges">
        {readings.map((r) => {
          const spec = specs[r.key];
          const inRange =
            spec && (spec.min !== null || spec.max !== null)
              ? (spec.min === null || r.value >= spec.min) &&
                (spec.max === null || r.value <= spec.max)
              : null;
          return (
            <div className="gauge" key={r.key}>
              <div className="gauge-name">{r.name}</div>
              <div className="gauge-value">
                {formatValue(r.value)}
                <span className="gauge-unit">{r.unit}</span>
              </div>
              {spec && (
                <div
                  className={`gauge-spec ${
                    inRange === null ? "" : inRange ? "gauge-spec-ok" : "gauge-spec-bad"
                  }`}
                  title={spec.condition}
                >
                  {spec.min !== null && spec.max !== null
                    ? `normal: ${formatValue(spec.min)}–${formatValue(spec.max)}`
                    : spec.target !== null
                      ? `target: ${formatValue(spec.target)}`
                      : spec.condition}
                </div>
              )}
            </div>
          );
        })}
      </div>
      {readings.length === 0 && !error && <p className="muted">Waiting for data…</p>}
    </div>
  );
}

function formatValue(v: number): string {
  if (Math.abs(v) >= 100) return v.toFixed(0);
  if (Math.abs(v) >= 10) return v.toFixed(1);
  return v.toFixed(2);
}
