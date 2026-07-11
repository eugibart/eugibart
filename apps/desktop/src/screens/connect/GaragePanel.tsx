import { useEffect, useState } from "react";
import { DefinitionInfo } from "../../ipc";
import { BikeProfile, describeMods } from "../../garage";

export default function GaragePanel({
  profiles,
  definitions,
  busy,
  onConnect,
  onEdit,
  onRemove,
}: {
  profiles: BikeProfile[];
  definitions: DefinitionInfo[];
  busy: boolean;
  onConnect: (profile: BikeProfile) => void;
  onEdit: (profile: BikeProfile) => void;
  onRemove: (id: string) => void;
}) {
  const [confirmingId, setConfirmingId] = useState<string | null>(null);

  useEffect(() => {
    if (!confirmingId) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setConfirmingId(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [confirmingId]);

  if (profiles.length === 0) {
    return (
      <p className="muted small">
        No bikes yet — use "Add your bike" below to save one for one-click connecting.
      </p>
    );
  }

  return (
    <ul className="garage-list">
      {profiles.map((p) => {
        const def = definitions.find((d) => d.id === p.definitionId);
        return (
          <li className="garage-card" key={p.id}>
            <div className="garage-card-head">
              <span className="garage-card-name">{p.name}</span>
              <span className="muted small">
                {p.brand} {p.model} ({p.year ?? "year unknown"})
              </span>
            </div>
            <div className="muted small">Mods: {describeMods(p.mods)}</div>
            {def && (
              <div className="muted small">
                ECU: {def.name}{" "}
                {def.verified ? (
                  <span className="badge badge-ok">verified</span>
                ) : (
                  <span className="badge badge-warn">unverified</span>
                )}
              </div>
            )}
            {confirmingId === p.id ? (
              <div className="confirm-box">
                <p>Remove {p.name} from your garage?</p>
                <div className="btn-row">
                  <button
                    className="btn btn-danger"
                    onClick={() => {
                      onRemove(p.id);
                      setConfirmingId(null);
                    }}
                  >
                    Remove
                  </button>
                  <button className="btn" onClick={() => setConfirmingId(null)}>
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <div className="btn-row">
                <button
                  className="btn btn-primary"
                  onClick={() => onConnect(p)}
                  disabled={busy}
                  aria-label={`Connect ${p.name}`}
                >
                  Connect
                </button>
                <button className="btn btn-small" onClick={() => onEdit(p)} disabled={busy}>
                  Edit
                </button>
                <button
                  className="btn btn-small"
                  onClick={() => setConfirmingId(p.id)}
                  disabled={busy}
                >
                  Delete
                </button>
              </div>
            )}
          </li>
        );
      })}
    </ul>
  );
}
