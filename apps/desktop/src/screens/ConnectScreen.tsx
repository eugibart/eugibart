import { useEffect, useState } from "react";
import { api, ConnectionInfo, DefinitionInfo, SIMULATOR_PORT } from "../ipc";

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
    try {
      onConnected(await api.connect(definitionId, port));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
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

      <button className="btn btn-primary" onClick={connect} disabled={busy || !definitionId}>
        {busy ? "Connecting…" : "Connect"}
      </button>

      <p className="muted small">
        Connecting is always read-only. Service operations require explicitly enabling
        service mode after connecting.
      </p>
    </div>
  );
}
