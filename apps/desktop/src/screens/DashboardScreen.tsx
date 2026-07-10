import { useEffect, useRef, useState } from "react";
import { api, Reading } from "../ipc";

const POLL_INTERVAL_MS = 500;

export default function DashboardScreen() {
  const [readings, setReadings] = useState<Reading[]>([]);
  const [error, setError] = useState<string | null>(null);
  const polling = useRef(false);

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
        {readings.map((r) => (
          <div className="gauge" key={r.key}>
            <div className="gauge-name">{r.name}</div>
            <div className="gauge-value">
              {formatValue(r.value)}
              <span className="gauge-unit">{r.unit}</span>
            </div>
          </div>
        ))}
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
