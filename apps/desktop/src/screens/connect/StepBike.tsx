import { useMemo, useState } from "react";
import { CatalogBikeInfo, DefinitionInfo } from "../../ipc";
import { brandsIn, modelsFor, yearBounds } from "./catalogHelpers";

export interface BikeSelection {
  brand: string;
  model: string;
  year: number;
  definitionId: string | null;
  fromCatalog: boolean;
}

export default function StepBike({
  catalog,
  definitions,
  value,
  onChange,
  onNext,
}: {
  catalog: CatalogBikeInfo[];
  definitions: DefinitionInfo[];
  value: BikeSelection;
  onChange: (v: BikeSelection) => void;
  onNext: () => void;
}) {
  const [manualPick, setManualPick] = useState(false);

  const brands = useMemo(() => brandsIn(catalog), [catalog]);
  const models = useMemo(() => modelsFor(catalog, value.brand), [catalog, value.brand]);
  const entry = models.find((m) => m.model === value.model) ?? null;

  const selectBrand = (brand: string) => {
    const first = modelsFor(catalog, brand)[0];
    setManualPick(false);
    onChange({
      brand,
      model: first?.model ?? "",
      year: first ? yearBounds(first).max : new Date().getFullYear(),
      definitionId: first?.definition_id ?? null,
      fromCatalog: true,
    });
  };

  const selectModel = (model: string) => {
    const found = models.find((m) => m.model === model);
    setManualPick(false);
    onChange({
      brand: value.brand,
      model,
      year: found ? yearBounds(found).max : value.year,
      definitionId: found?.definition_id ?? null,
      fromCatalog: true,
    });
  };

  const selectYear = (year: number) => {
    onChange({ ...value, year });
  };

  const selectManualDefinition = (definitionId: string) => {
    onChange({ ...value, definitionId: definitionId || null, fromCatalog: false });
  };

  const resolvedDef = value.definitionId
    ? definitions.find((d) => d.id === value.definitionId)
    : null;
  const canProceed = value.definitionId !== null;

  return (
    <div>
      <h3 tabIndex={-1} id="wizard-step-heading">
        Step 1 of 3 — Your bike
      </h3>

      {!manualPick && (
        <>
          <div className="form-row">
            <label htmlFor="wiz-brand">Brand</label>
            <select id="wiz-brand" value={value.brand} onChange={(e) => selectBrand(e.target.value)}>
              {brands.map((b) => (
                <option key={b} value={b}>
                  {b}
                </option>
              ))}
            </select>
          </div>

          <div className="form-row">
            <label htmlFor="wiz-model">Model</label>
            <select id="wiz-model" value={value.model} onChange={(e) => selectModel(e.target.value)}>
              {models.map((m) => (
                <option key={m.model} value={m.model}>
                  {m.model}
                  {m.variants.length > 0 ? ` (${m.variants.join("/")})` : ""}
                </option>
              ))}
            </select>
          </div>

          {entry && (
            <div className="form-row">
              <label htmlFor="wiz-year">Year</label>
              <input
                id="wiz-year"
                type="number"
                min={yearBounds(entry).min}
                max={yearBounds(entry).max}
                value={value.year}
                onChange={(e) => selectYear(Number(e.target.value))}
              />
            </div>
          )}

          {resolvedDef && (
            <p className="muted small">
              ECU: {resolvedDef.name}{" "}
              {resolvedDef.verified ? (
                <span className="badge badge-ok">verified</span>
              ) : (
                <span className="badge badge-warn">unverified definition</span>
              )}
            </p>
          )}

          {entry?.gap_note && (
            <div className="error-box" role="alert">
              <p style={{ marginTop: 0 }}>We don't have this bike yet.</p>
              <p>{entry.gap_note}</p>
              <button className="btn btn-small" onClick={() => setManualPick(true)}>
                Choose an ECU definition manually
              </button>
            </div>
          )}

          <p className="muted small">
            <button className="btn btn-small" onClick={() => setManualPick(true)}>
              My bike isn't listed
            </button>
          </p>
        </>
      )}

      {manualPick && (
        <div className="form-row">
          <label htmlFor="wiz-manual-def">ECU definition</label>
          <select
            id="wiz-manual-def"
            value={value.definitionId ?? ""}
            onChange={(e) => selectManualDefinition(e.target.value)}
          >
            <option value="">Choose one…</option>
            {definitions.map((d) => (
              <option key={d.id} value={d.id}>
                {d.manufacturer} — {d.name}
              </option>
            ))}
          </select>
          <button className="btn btn-small" onClick={() => setManualPick(false)}>
            Back to catalog
          </button>
        </div>
      )}

      <div className="btn-row">
        <button className="btn btn-primary" onClick={onNext} disabled={!canProceed}>
          Next
        </button>
      </div>
    </div>
  );
}
