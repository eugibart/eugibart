import { useEffect, useRef, useState } from "react";
import { api, SIMULATOR_PORT, VacuumStatus } from "../ipc";
import ComparePanel from "../ComparePanel";
import { makeSnapshot, useSnapshots } from "../snapshots";

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
  const { addSnapshot } = useSnapshots();
  const [snapshotLabel, setSnapshotLabel] = useState("");
  const [snapshotSaved, setSnapshotSaved] = useState<string | null>(null);

  const captureSnapshot = async () => {
    const label = snapshotLabel.trim() || "snapshot";
    // Grab a fresh full reading set when the ECU is up, so the snapshot
    // carries rpm/CO-relevant channels alongside the vacuum picture.
    let readings: Awaited<ReturnType<typeof api.pollLiveData>> = [];
    if (ecuConnected) {
      try {
        readings = await api.pollLiveData();
      } catch {
        // Vacuum-only snapshot is still worth keeping.
      }
    }
    addSnapshot(makeSnapshot(label, null, readings, status));
    setSnapshotLabel("");
    setSnapshotSaved(label);
  };

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
            <label className="visually-hidden" htmlFor="sync-snapshot-label">
              Snapshot label
            </label>
            <input
              id="sync-snapshot-label"
              className="snapshot-label-input"
              placeholder="e.g. after sync"
              value={snapshotLabel}
              onChange={(e) => setSnapshotLabel(e.target.value)}
            />
            <button className="btn btn-small" onClick={captureSnapshot} disabled={!status}>
              Snapshot
            </button>
            <button className="btn btn-small" onClick={disconnect}>
              Disconnect gauge
            </button>
          </div>
          {snapshotSaved && (
            <p className="muted small" role="status">
              Snapshot "{snapshotSaved}" saved — compare below.
            </p>
          )}

          {status && (
            <>
              <div className={`sync-spread ${spreadClass}`}>
                Spread: <strong>{status.spread_kpa.toFixed(2)} kPa</strong>
                {status.spread_kpa < SPREAD_GOOD
                  ? " — balanced"
                  : status.spread_kpa < SPREAD_OK
                    ? " — close, keep going"
                    : " — unbalanced"}
                <span className="sync-spread-goal">
                  balanced when spread &lt; {SPREAD_GOOD.toFixed(1)} kPa
                </span>
              </div>

              <div className="sync-bars">
                {(() => {
                  // Mark the cylinder(s) actually breaking the balance: the
                  // furthest from the mean, only once the spread says
                  // something is wrong. Deltas are shown vs cyl 1, but the
                  // odd one out is measured from the mean — otherwise the
                  // reference cylinder could never be the culprit.
                  const mean =
                    status.channels_kpa.reduce((s, v) => s + v, 0) /
                    Math.max(1, status.channels_kpa.length);
                  const devs = status.channels_kpa.map((v) => Math.abs(v - mean));
                  const maxDev = Math.max(...devs);
                  const unbalanced = status.spread_kpa >= SPREAD_OK;
                  return status.channels_kpa.map((kpa, i) => {
                    const pct = Math.min(
                      100,
                      Math.max(4, ((kpa - KPA_MIN) / (KPA_MAX - KPA_MIN)) * 100),
                    );
                    const delta = status.deltas_kpa[i] ?? 0;
                    const offender = unbalanced && maxDev > 0 && devs[i] >= maxDev * 0.9;
                    return (
                      <div className="sync-cyl" key={i}>
                        <div className="sync-bar-track">
                          <div
                            className={`sync-bar-fill ${offender ? "sync-bar-fill-off" : ""}`}
                            style={{ height: `${pct}%` }}
                          />
                        </div>
                        <div className="sync-kpa mono">{kpa.toFixed(1)}</div>
                        <div className={`sync-delta mono ${offender ? "sync-delta-off" : ""}`}>
                          {i === 0 ? "ref" : `${delta >= 0 ? "+" : ""}${delta.toFixed(2)}`}
                        </div>
                        <div className="sync-label">
                          Cyl {i + 1}
                          {offender && <span className="visually-hidden"> — furthest out</span>}
                        </div>
                      </div>
                    );
                  });
                })()}
              </div>
              <p className="muted small">kPa absolute — lower bar = stronger vacuum.</p>
            </>
          )}
        </>
      )}

      <h3 className="section-gap">Before / after compare</h3>
      <p className="muted small">
        Capture a snapshot before touching the screws and another after — the deltas below are
        the evidence that the adjustment actually improved things.
      </p>
      <ComparePanel />

      <h3 className="section-gap">Procedure</h3>
      <ol className="pre-list">
        {PROCEDURE.map((step, i) => (
          <li key={i}>{step}</li>
        ))}
      </ol>
    </div>
  );
}
