import { useState } from "react";
import { compareSnapshots, useSnapshots } from "./snapshots";

/**
 * Before/after comparison of two session snapshots — the evidence view for
 * sync and CO work: pick the snapshot you took before touching anything and
 * the one after, and read the deltas instead of trusting memory.
 */
export default function ComparePanel() {
  const { snapshots, removeSnapshot, max } = useSnapshots();
  const [aId, setAId] = useState("");
  const [bId, setBId] = useState("");

  const a = snapshots.find((s) => s.id === aId) ?? null;
  const b = snapshots.find((s) => s.id === bId) ?? null;
  const rows = a && b ? compareSnapshots(a, b) : null;

  const describe = (s: (typeof snapshots)[number]) =>
    `${s.label} — ${new Date(s.atMs).toLocaleTimeString()}${s.profileName ? ` (${s.profileName})` : ""}`;

  const fmt = (v: number | null) => (v === null ? "—" : v.toFixed(2));
  const fmtDelta = (v: number | null) =>
    v === null ? "—" : `${v >= 0 ? "+" : ""}${v.toFixed(2)}`;

  if (snapshots.length === 0) {
    return (
      <p className="muted small">
        No snapshots yet — capture one before you start adjusting (Dashboard or here), another
        after, and compare them side by side.
      </p>
    );
  }

  return (
    <div>
      <div className="form-row">
        <label htmlFor="compare-a">Before</label>
        <select id="compare-a" value={aId} onChange={(e) => setAId(e.target.value)}>
          <option value="">Choose a snapshot…</option>
          {snapshots.map((s) => (
            <option key={s.id} value={s.id}>
              {describe(s)}
            </option>
          ))}
        </select>
      </div>
      <div className="form-row">
        <label htmlFor="compare-b">After</label>
        <select id="compare-b" value={bId} onChange={(e) => setBId(e.target.value)}>
          <option value="">Choose a snapshot…</option>
          {snapshots.map((s) => (
            <option key={s.id} value={s.id}>
              {describe(s)}
            </option>
          ))}
        </select>
      </div>

      {rows && a && b && (
        <div className="compare-scroll">
          <table className="table">
            <caption className="visually-hidden">
              Comparison of snapshot {a.label} against snapshot {b.label}
            </caption>
            <thead>
              <tr>
                <th>Metric</th>
                <th>Before</th>
                <th>After</th>
                <th>Δ</th>
                <th>Unit</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.label}>
                  <td>{row.label}</td>
                  <td className="mono">{fmt(row.a)}</td>
                  <td className="mono">{fmt(row.b)}</td>
                  <td className="mono">{fmtDelta(row.delta)}</td>
                  <td className="muted">{row.unit}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <details>
        <summary>Manage snapshots ({snapshots.length}/{max})</summary>
        <ul className="snapshot-list">
          {snapshots.map((s) => (
            <li key={s.id}>
              <span>{describe(s)}</span>{" "}
              <button
                className="btn btn-small"
                onClick={() => removeSnapshot(s.id)}
                aria-label={`Delete snapshot ${s.label}`}
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      </details>
    </div>
  );
}
