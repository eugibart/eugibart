import { DefinitionInfo, ModGuidanceInfo } from "../../ipc";
import { BikeMods, describeMods } from "../../garage";
import { modsMatch } from "../../specResolution";
import { BikeSelection } from "./StepBike";
import WizardFooter from "./WizardFooter";

export default function StepSummary({
  selection,
  mods,
  name,
  onNameChange,
  definitions,
  guidance,
  onBack,
  onSave,
  onCancel,
  busy,
}: {
  selection: BikeSelection;
  mods: BikeMods;
  name: string;
  onNameChange: (v: string) => void;
  definitions: DefinitionInfo[];
  guidance: ModGuidanceInfo | null;
  onBack: () => void;
  onSave: (alsoConnect: boolean) => void;
  onCancel: () => void;
  busy: boolean;
}) {
  const def = definitions.find((d) => d.id === selection.definitionId);
  const hasAdjustment =
    guidance !== null &&
    selection.definitionId !== null &&
    guidance.adjustments.some(
      (a) => a.definition_id === selection.definitionId && modsMatch(a.requires, mods),
    );

  return (
    <div>
      <h3 tabIndex={-1} id="wizard-step-heading">
        Step 3 of 3 — Summary
      </h3>

      <div className="form-row">
        <label htmlFor="wiz-name">Name this bike</label>
        <input
          id="wiz-name"
          value={name}
          onChange={(e) => onNameChange(e.target.value)}
          placeholder={`${selection.brand} ${selection.model}`}
        />
      </div>

      <div className="def-details">
        <div>
          {selection.brand} {selection.model} ({selection.year})
        </div>
        <div>
          <span className="muted">Modifications:</span> {describeMods(mods)}
        </div>
        {def && (
          <div>
            <span className="muted">ECU:</span> {def.name}{" "}
            {def.verified ? (
              <span className="badge badge-ok">verified</span>
            ) : (
              <span className="badge badge-warn">unverified definition</span>
            )}
          </div>
        )}
      </div>

      {hasAdjustment && (
        <p className="muted small">
          Because of your mods, some reference ranges will use community figures instead of the
          factory ones. These are <strong>unverified</strong> — treat them as a starting point, not
          a spec. You can always type in your own target once connected.
        </p>
      )}

      <WizardFooter onCancel={onCancel} onBack={onBack} busy={busy}>
        <button className="btn" onClick={() => onSave(false)} disabled={busy || !name.trim()}>
          Save to garage
        </button>
        <button
          className="btn btn-primary"
          onClick={() => onSave(true)}
          disabled={busy || !name.trim()}
        >
          Save and connect
        </button>
      </WizardFooter>
    </div>
  );
}
