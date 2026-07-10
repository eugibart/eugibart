import { useEffect, useState } from "react";
import { api, ConnectionInfo, RoutineInfo } from "../ipc";

export default function ServiceScreen({
  connection,
  onStatusChange,
}: {
  connection: ConnectionInfo;
  onStatusChange: () => void;
}) {
  const [routines, setRoutines] = useState<RoutineInfo[]>([]);
  const [confirmKey, setConfirmKey] = useState<string | null>(null);
  const [confirmText, setConfirmText] = useState("");
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    api
      .listDefinitions()
      .then((defs) => {
        const def = defs.find((d) => d.id === connection.definition_id);
        setRoutines(def?.routines ?? []);
      })
      .catch((e) => setError(String(e)));
  }, [connection.definition_id]);

  const enableServiceMode = async () => {
    await api.enableServiceMode();
    onStatusChange();
  };

  const run = async (key: string) => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const response = await api.runRoutine(key, true);
      setResult(`Routine completed. ECU response: ${response}`);
      setConfirmKey(null);
      setConfirmText("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const confirming = routines.find((r) => r.key === confirmKey);

  return (
    <div className="panel">
      <h2>Service functions</h2>

      {!connection.service_mode && (
        <div className="confirm-box">
          <p>
            This session is <strong>read-only</strong>. Service functions send commands that
            change ECU state (actuator tests, adaptation resets). Enable service mode only
            with the bike on a stand, in neutral, and follow each function's preconditions.
          </p>
          <button className="btn btn-warn" onClick={enableServiceMode}>
            Enable service mode for this session
          </button>
        </div>
      )}

      {error && <div className="error-box">{error}</div>}
      {result && <div className="ok-box">{result}</div>}

      <div className="routine-list">
        {routines.map((r) => (
          <div className="routine" key={r.key}>
            <div className="routine-head">
              <span className="routine-name">{r.name}</span>
              <span className={`badge badge-risk-${r.risk}`}>{r.risk} risk</span>
              {!r.verified && <span className="badge badge-warn">unverified</span>}
            </div>
            {r.description && <p className="muted small">{r.description}</p>}
            {r.preconditions.length > 0 && (
              <ul className="pre-list">
                {r.preconditions.map((p, i) => (
                  <li key={i}>{p}</li>
                ))}
              </ul>
            )}
            <button
              className="btn"
              disabled={!connection.service_mode || busy}
              onClick={() => {
                setConfirmKey(r.key);
                setConfirmText("");
                setResult(null);
                setError(null);
              }}
            >
              Run…
            </button>
          </div>
        ))}
      </div>

      {confirming && (
        <div className="confirm-box">
          <p>
            Run <strong>{confirming.name}</strong>? Preconditions are re-checked against live
            data before anything is sent. Type <code>RUN</code> to confirm.
          </p>
          <input
            value={confirmText}
            onChange={(e) => setConfirmText(e.target.value)}
            placeholder="Type RUN"
          />
          <div className="btn-row">
            <button
              className="btn btn-danger"
              disabled={confirmText !== "RUN" || busy}
              onClick={() => run(confirming.key)}
            >
              {busy ? "Running…" : "Run routine"}
            </button>
            <button className="btn" onClick={() => setConfirmKey(null)}>
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
