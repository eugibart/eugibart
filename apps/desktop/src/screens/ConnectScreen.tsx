import { useEffect, useState } from "react";
import { api, ConnectionInfo, DefinitionInfo, SIMULATOR_PORT, TroubleshootStep } from "../ipc";

export default function ConnectScreen({
  onConnected,
}: {
  onConnected: (info: ConnectionInfo) => void;
}) {
  const [definitions, setDefinitions] = useState<DefinitionInfo[]>([]);
  const [ports, setPorts] = useState<string[]>([]);
  const [definitionId, setDefinitionId] = useState("");
  const [port, setPort] = useState(SIMULATOR_PORT);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [troubleshooting, setTroubleshooting] = useState(false);
  const [steps, setSteps] = useState<TroubleshootStep[] | null>(null);

  const refresh = async () => {
    const [defs, portList] = await Promise.all([
      api.listDefinitions(),
      api.listSerialPorts(),
    ]);
    setDefinitions(defs);
    setPorts(portList);
    if (!definitionId && defs.length > 0) {
      const brutale = defs.find((d) => d.id.includes("brutale")) ?? defs[0];
      setDefinitionId(brutale.id);
    }
  };

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selected = definitions.find((d) => d.id === definitionId);

  const connect = async () => {
    setBusy(true);
    setError(null);
    setSteps(null);
    try {
      onConnected(await api.connect(definitionId, port));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const troubleshoot = async () => {
    setTroubleshooting(true);
    setSteps(null);
    setError(null);
    try {
      setSteps(await api.troubleshootConnection(definitionId, port));
    } catch (e) {
      setError(String(e));
    } finally {
      setTroubleshooting(false);
    }
  };

  return (
    <div className="panel">
      <h2>Connect to a bike</h2>

      <div className="form-row">
        <label>ECU definition</label>
        <select value={definitionId} onChange={(e) => setDefinitionId(e.target.value)}>
          {definitions.map((d) => (
            <option key={d.id} value={d.id}>
              {d.manufacturer} — {d.name}
            </option>
          ))}
        </select>
      </div>

      {selected && (
        <div className="def-details">
          <div>
            <span className="muted">Bus:</span> {selected.bus}
            {"  "}
            {selected.verified ? (
              <span className="badge badge-ok">verified</span>
            ) : (
              <span className="badge badge-warn">unverified definition</span>
            )}
          </div>
          {selected.models.length > 0 && (
            <div>
              <span className="muted">Models:</span> {selected.models.join(", ")}
            </div>
          )}
          {selected.notes && <p className="notes">{selected.notes}</p>}
        </div>
      )}

      <div className="form-row">
        <label>Port</label>
        <select value={port} onChange={(e) => setPort(e.target.value)}>
          {ports.map((p) => (
            <option key={p} value={p}>
              {p === SIMULATOR_PORT ? "Built-in ECU simulator (no hardware)" : p}
            </option>
          ))}
        </select>
        <button className="btn btn-small" onClick={() => refresh()}>
          ↻ Refresh
        </button>
      </div>

      {error && <div className="error-box">{error}</div>}

      <div className="btn-row">
        <button className="btn btn-primary" onClick={connect} disabled={busy || !definitionId}>
          {busy ? "Connecting…" : "Connect"}
        </button>
        <button
          className="btn"
          onClick={troubleshoot}
          disabled={troubleshooting || !definitionId}
          title="Step-by-step check of cable, port, wiring, and ECU handshake"
        >
          {troubleshooting ? "Testing…" : "Troubleshoot connection"}
        </button>
      </div>

      {steps && (
        <div className="ts-panel">
          <h3>Connection check</h3>
          {steps.map((s) => (
            <div className={`ts-step ts-${s.status}`} key={s.name}>
              <div className="ts-head">
                <span className="ts-icon">
                  {s.status === "passed" ? "✓" : s.status === "failed" ? "✗" : "○"}
                </span>
                <span className="ts-name">{s.name}</span>
              </div>
              <div className="ts-detail">{s.detail}</div>
              {s.suggestion && <div className="ts-suggestion">→ {s.suggestion}</div>}
            </div>
          ))}
        </div>
      )}

      <p className="muted small">
        Connecting is always read-only. Service operations require explicitly enabling
        service mode after connecting.
      </p>
    </div>
  );
}
