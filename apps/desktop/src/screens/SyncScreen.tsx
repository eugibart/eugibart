import { useEffect, useRef, useState } from "react";
import { api, SIMULATOR_PORT, VacuumStatus } from "../ipc";

const POLL_INTERVAL_MS = 300;
// Indicative thresholds (kPa spread across cylinders) — workshop manuals
// vary; well-balanced bodies typically sit within ~0.5 kPa of each other.
const SPREAD_GOOD = 0.5;
const SPREAD_OK = 1.5;

// Bar scale: idling manifold pressure lives roughly in this window.
const KPA_MIN = 20;
const KPA_MAX = 45;

const PROCEDURE = [
  "Warm the engine to operating temperature, then shut it off",
  "Connect the gauge hoses to the vacuum ports on each throttle body",
  "Start the engine and let it idle",
  "Balance cylinder pair 1–2 with their bypass screw, watching the bars",
  "Balance pair 3–4 the same way",
  "Use the center screw to bring the pairs together (spread → green)",
  "Only now adjust idle CO — sync first, CO after",
];

export default function SyncScreen({ ecuConnected }: { ecuConnected: boolean }) {
  const [gaugePort, setGaugePort] = useState<string | null>(null);
  const [ports, setPorts] = useState<string[]>([]);
  const [selectedPort, setSelectedPort] = useState(SIMULATOR_PORT);
  const [status, setStatus] = useState<VacuumStatus | null>(null);
  const [rpm, setRpm] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  // WCAG 2.2.2: auto-updating content needs a pause control.
  const [paused, setPaused] = useState(false);
  const polling = useRef(false);

  useEffect(() => {
    api.listSerialPorts().then(setPorts).catch(() => {});
    api
      .vacuumStatus()
      .then((v) => setGaugePort(v?.port ?? null))
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!gaugePort || paused) return;
    let cancelled = false;
    const tick = async () => {
      if (polling.current) return;
      polling.current = true;
      try {
        const s = await api.pollVacuum();
        if (!cancelled) {
          setStatus(s);
          setError(null);
        }
        if (ecuConnected) {
          const readings = await api.pollLiveData();
          const r = readings.find((x) => x.key === "rpm");
          if (!cancelled) setRpm(r ? r.value : null);
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
  }, [gaugePort, ecuConnected, paused]);

  const connect = async () => {
    setError(null);
    try {
      const info = await api.connectVacuum(selectedPort);
      setGaugePort(info.port);
    } catch (e) {
      setError(String(e));
    }
  };

  const disconnect = async () => {
    await api.disconnectVacuum();
    setGaugePort(null);
    setStatus(null);
  };

  const spreadClass =
    status == null
      ? ""
      : status.spread_kpa < SPREAD_GOOD
        ? "sync-good"
        : status.spread_kpa < SPREAD_OK
          ? "sync-ok"
          : "sync-bad";

  return (
    <div className="panel panel-wide">
      <h2>Throttle-body sync (vacuometro)</h2>
      <p className="muted small">
        Balance the throttle bodies first — only then is a CO adjustment meaningful.
        Requires a digital vacuum gauge on a second USB port (build one for ~€50: see
        docs/VACUOMETRO.md).
      </p>

      {error && (
        <div className="error-box" role="alert">
          {error}
        </div>
      )}

      {!gaugePort ? (
        <div className="form-row">
          <label htmlFor="gauge-port">Gauge port</label>
          <select
            id="gauge-port"
            value={selectedPort}
            onChange={(e) => setSelectedPort(e.target.value)}
          >
            {ports.map((p) => (
              <option key={p} value={p}>
                {p === SIMULATOR_PORT ? "Simulated vacuum gauge (no hardware)" : p}
              </option>
            ))}
          </select>
          <button className="btn btn-primary" onClick={connect}>
            Connect gauge
          </button>
        </div>
      ) : (
        <>
          <div className="sync-topline">
            <span className="muted small">
              Gauge on <span className="mono">{gaugePort}</span>
            </span>
            {rpm !== null && (
              <span className="sync-rpm mono">{rpm.toFixed(0)} rpm</span>
            )}
            {!ecuConnected && (
              <span className="muted small">
                (connect the ECU too to see live RPM here)
              </span>
            )}
            <button
              className="btn btn-small"
              onClick={() => setPaused((p) => !p)}
              aria-pressed={paused}
            >
              {paused ? "Resume updates" : "Pause updates"}
            </button>
            <button className="btn btn-small" onClick={disconnect}>
              Disconnect gauge
            </button>
          </div>

          {status && (
            <>
              <div className={`sync-spread ${spreadClass}`}>
                Spread: <strong>{status.spread_kpa.toFixed(2)} kPa</strong>
                {status.spread_kpa < SPREAD_GOOD
                  ? " — balanced"
                  : status.spread_kpa < SPREAD_OK
                    ? " — close, keep going"
                    : " — unbalanced"}
              </div>

              <div className="sync-bars">
                {status.channels_kpa.map((kpa, i) => {
                  const pct = Math.min(
                    100,
                    Math.max(4, ((kpa - KPA_MIN) / (KPA_MAX - KPA_MIN)) * 100),
                  );
                  const delta = status.deltas_kpa[i] ?? 0;
                  return (
                    <div className="sync-cyl" key={i}>
                      <div className="sync-bar-track">
                        <div className="sync-bar-fill" style={{ height: `${pct}%` }} />
                      </div>
                      <div className="sync-kpa mono">{kpa.toFixed(1)}</div>
                      <div className="sync-delta mono">
                        {i === 0 ? "ref" : `${delta >= 0 ? "+" : ""}${delta.toFixed(2)}`}
                      </div>
                      <div className="sync-label">Cyl {i + 1}</div>
                    </div>
                  );
                })}
              </div>
              <p className="muted small">kPa absolute — lower bar = stronger vacuum.</p>
            </>
          )}
        </>
      )}

      <h3 className="section-gap">Procedure</h3>
      <ol className="pre-list">
        {PROCEDURE.map((step, i) => (
          <li key={i}>{step}</li>
        ))}
      </ol>
    </div>
  );
}
