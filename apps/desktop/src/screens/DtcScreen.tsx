import { useEffect, useState } from "react";
import { api, Dtc } from "../ipc";

export default function DtcScreen({ serviceMode }: { serviceMode: boolean }) {
  const [dtcs, setDtcs] = useState<Dtc[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  const read = async () => {
    setBusy(true);
    setError(null);
    try {
      setDtcs(await api.readDtcs());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    read();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const clear = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.clearDtcs(true);
      setConfirming(false);
      await read();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="panel">
      <h2>Fault codes</h2>
      {error && <div className="error-box">{error}</div>}

      {dtcs && dtcs.length === 0 && <p className="ok-box">No stored fault codes.</p>}

      {dtcs && dtcs.length > 0 && (
        <table className="table">
          <thead>
            <tr>
              <th>Code</th>
              <th>Status</th>
              <th>Description</th>
            </tr>
          </thead>
          <tbody>
            {dtcs.map((d) => (
              <tr key={d.code}>
                <td className="mono">{d.code.toString(16).toUpperCase().padStart(4, "0")}</td>
                <td className="mono">0x{d.status.toString(16).toUpperCase().padStart(2, "0")}</td>
                <td>{d.description ?? <span className="muted">unknown code</span>}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div className="btn-row">
        <button className="btn" onClick={read} disabled={busy}>
          ↻ Re-read
        </button>
        {dtcs && dtcs.length > 0 && !confirming && (
          <button
            className="btn btn-danger"
            onClick={() => setConfirming(true)}
            disabled={busy || !serviceMode}
            title={serviceMode ? "" : "Enable service mode in the Service tab first"}
          >
            Clear fault codes…
          </button>
        )}
      </div>

      {!serviceMode && dtcs && dtcs.length > 0 && (
        <p className="muted small">
          Clearing codes changes ECU state — enable service mode in the Service tab first.
        </p>
      )}

      {confirming && (
        <div className="confirm-box">
          <p>
            Clear all stored fault codes? The fault history will be lost — record the codes
            above first if you need them.
          </p>
          <div className="btn-row">
            <button className="btn btn-danger" onClick={clear} disabled={busy}>
              Yes, clear codes
            </button>
            <button className="btn" onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
