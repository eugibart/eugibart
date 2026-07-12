import { useEffect, useState } from "react";
import { api, ConnectionInfo, ModGuidanceInfo, RoutineInfo } from "../ipc";
import { BikeProfile } from "../garage";
import { applicableProcedureNotes } from "../specResolution";
import SourceLink from "../SourceLink";

export default function ServiceScreen({
  connection,
  onStatusChange,
  activeProfile,
}: {
  connection: ConnectionInfo;
  onStatusChange: () => void;
  activeProfile: BikeProfile | null;
}) {
  const [routines, setRoutines] = useState<RoutineInfo[]>([]);
  const [guidance, setGuidance] = useState<ModGuidanceInfo | null>(null);
  const [confirmKey, setConfirmKey] = useState<string | null>(null);
  const [confirmText, setConfirmText] = useState("");
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    Promise.all([api.listDefinitions(), api.listModGuidance()])
      .then(([defs, guidanceInfo]) => {
        const def = defs.find((d) => d.id === connection.definition_id);
        setRoutines(def?.routines ?? []);
        setGuidance(guidanceInfo);
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

  useEffect(() => {
    if (!confirmKey) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setConfirmKey(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [confirmKey]);

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

      {error && (
        <div className="error-box" role="alert">
          {error}
        </div>
      )}
      {result && (
        <div className="ok-box" role="status">
          {result}
        </div>
      )}

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
              <>
                <span className="routine-subhead">Preconditions</span>
                <ul className="pre-list">
                  {r.preconditions.map((p, i) => (
                    <li key={i}>{p}</li>
                  ))}
                </ul>
              </>
            )}
            {r.procedure.length > 0 && (
              // Collapsed by default so the routine list scans; preconditions
              // stay always-visible above because they gate safety.
              <details className="section-disclosure">
                <summary>
                  Procedure — {r.procedure.length} step{r.procedure.length === 1 ? "" : "s"}
                </summary>
                <ol className="pre-list">
                  {r.procedure.map((step, i) => (
                    <li key={i}>{step}</li>
                  ))}
                </ol>
              </details>
            )}
            {applicableProcedureNotes(connection.definition_id, guidance, activeProfile, r.key).map(
              (n, i) => (
                <div className="mod-note" key={i}>
                  <span className="mod-note-label">For your mods — community, unverified</span>
                  {n.note}
                  <div className="mod-note-source">
                    <SourceLink site={n.source} url={n.sourceUrl} />
                  </div>
                </div>
              ),
            )}
            <div className="routine-footer">
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
          </div>
        ))}
      </div>

      {confirming && (
        <div className="confirm-box" role="group" aria-labelledby="confirm-run-label">
          <p id="confirm-run-label">
            Run <strong>{confirming.name}</strong>? Preconditions are re-checked against live
            data before anything is sent. Type <code>RUN</code> to confirm.
          </p>
          <label className="visually-hidden" htmlFor="confirm-run-input">
            Type RUN to confirm
          </label>
          <input
            id="confirm-run-input"
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
