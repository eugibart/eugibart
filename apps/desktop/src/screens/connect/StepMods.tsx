import { BikeMods, STOCK_MODS } from "../../garage";
import WizardFooter from "./WizardFooter";

export default function StepMods({
  value,
  onChange,
  onNext,
  onBack,
  onCancel,
}: {
  value: BikeMods;
  onChange: (v: BikeMods) => void;
  onNext: () => void;
  onBack: () => void;
  onCancel: () => void;
}) {
  const modified = value.exhaust !== "stock" || value.eprom !== "stock" || value.airFilter !== "stock";

  const setStockOrModified = (isModified: boolean) => {
    onChange(isModified ? { ...value, exhaust: "slip-on-open" } : STOCK_MODS);
  };

  return (
    <div>
      <h3 tabIndex={-1} id="wizard-step-heading">
        Step 2 of 3 — Your modifications
      </h3>

      <fieldset className="fieldset">
        <legend>Is it stock or modified?</legend>
        <label className="radio-row">
          <input
            type="radio"
            name="wiz-stock-modified"
            checked={!modified}
            onChange={() => setStockOrModified(false)}
          />
          Stock
        </label>
        <label className="radio-row">
          <input
            type="radio"
            name="wiz-stock-modified"
            checked={modified}
            onChange={() => setStockOrModified(true)}
          />
          Modified
        </label>
      </fieldset>

      {modified && (
        <>
          <fieldset className="fieldset">
            <legend>Exhaust</legend>
            {(
              [
                ["stock", "Stock"],
                ["slip-on-open", "Open slip-ons"],
                ["full-system", "Full system"],
              ] as const
            ).map(([val, label]) => (
              <div key={val}>
                <label className="radio-row">
                  <input
                    type="radio"
                    name="wiz-exhaust"
                    checked={value.exhaust === val}
                    onChange={() => onChange({ ...value, exhaust: val })}
                  />
                  {label}
                </label>
                {/* Material belongs to the chosen exhaust — render it right
                    under the selected option, not after the whole list. */}
                {val !== "stock" && value.exhaust === val && (
                  <div className="radio-row-indent">
                    <span className="muted small">Material: </span>
                    {(["inox", "titanium"] as const).map((mat) => (
                      <label className="radio-row-inline" key={mat}>
                        <input
                          type="radio"
                          name="wiz-exhaust-material"
                          checked={value.exhaustMaterial === mat}
                          onChange={() => onChange({ ...value, exhaustMaterial: mat })}
                        />
                        {mat}
                      </label>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </fieldset>

          <fieldset className="fieldset">
            <legend>ECU / EPROM</legend>
            {(
              [
                ["stock", "Stock"],
                ["dealer-eprom", "Dedicated dealer EPROM"],
                ["custom-map", "Custom map"],
              ] as const
            ).map(([val, label]) => (
              <label className="radio-row" key={val}>
                <input
                  type="radio"
                  name="wiz-eprom"
                  checked={value.eprom === val}
                  onChange={() => onChange({ ...value, eprom: val })}
                />
                {label}
              </label>
            ))}
          </fieldset>

          <fieldset className="fieldset">
            <legend>Air filter</legend>
            {(
              [
                ["stock", "Stock"],
                ["high-flow", "High-flow"],
              ] as const
            ).map(([val, label]) => (
              <label className="radio-row" key={val}>
                <input
                  type="radio"
                  name="wiz-filter"
                  checked={value.airFilter === val}
                  onChange={() => onChange({ ...value, airFilter: val })}
                />
                {label}
              </label>
            ))}
          </fieldset>

          <div className="form-row">
            <label htmlFor="wiz-other-notes">Anything else?</label>
            <textarea
              id="wiz-other-notes"
              rows={2}
              value={value.otherNotes}
              onChange={(e) => onChange({ ...value, otherNotes: e.target.value })}
              placeholder="e.g. rejetted carbs, different cams…"
            />
          </div>
        </>
      )}

      <WizardFooter onCancel={onCancel} onBack={onBack}>
        <button className="btn btn-primary" onClick={onNext}>
          Next
        </button>
      </WizardFooter>
    </div>
  );
}
