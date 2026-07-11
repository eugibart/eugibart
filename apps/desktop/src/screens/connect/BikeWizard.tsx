import { useEffect, useRef, useState } from "react";
import { CatalogBikeInfo, DefinitionInfo, ModGuidanceInfo } from "../../ipc";
import { BikeMods, BikeProfile, newProfile, STOCK_MODS } from "../../garage";
import StepBike, { BikeSelection } from "./StepBike";
import StepMods from "./StepMods";
import StepSummary from "./StepSummary";

type Step = "bike" | "mods" | "summary";

function initialSelection(catalog: CatalogBikeInfo[], profile?: BikeProfile): BikeSelection {
  if (profile) {
    return {
      brand: profile.brand,
      model: profile.model,
      year: profile.year ?? new Date().getFullYear(),
      definitionId: profile.definitionId,
      fromCatalog: profile.fromCatalog,
    };
  }
  const first = catalog[0];
  return {
    brand: first?.brand ?? "",
    model: first?.model ?? "",
    year: first?.year_to ?? new Date().getFullYear(),
    definitionId: first?.definition_id ?? null,
    fromCatalog: true,
  };
}

export default function BikeWizard({
  catalog,
  definitions,
  guidance,
  initialProfile,
  onCancel,
  onSave,
}: {
  catalog: CatalogBikeInfo[];
  definitions: DefinitionInfo[];
  guidance: ModGuidanceInfo | null;
  initialProfile?: BikeProfile;
  onCancel: () => void;
  onSave: (profile: BikeProfile, alsoConnect: boolean) => void;
}) {
  const [step, setStep] = useState<Step>("bike");
  const [selection, setSelection] = useState<BikeSelection>(() =>
    initialSelection(catalog, initialProfile),
  );
  const [mods, setMods] = useState<BikeMods>(initialProfile?.mods ?? STOCK_MODS);
  const [name, setName] = useState(initialProfile?.name ?? "");
  const [busy, setBusy] = useState(false);

  const headingRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    // WCAG 2.4.3/3.2.2: moving to a new step shifts focus to its heading so
    // screen-reader users get the new step announced without hunting for it.
    headingRef.current?.querySelector("h3")?.focus();
  }, [step]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onCancel]);

  const save = (alsoConnect: boolean) => {
    if (!selection.definitionId) return;
    setBusy(true);
    const profile = initialProfile
      ? {
          ...initialProfile,
          name: name.trim(),
          brand: selection.brand,
          model: selection.model,
          year: selection.year,
          definitionId: selection.definitionId,
          fromCatalog: selection.fromCatalog,
          mods,
        }
      : newProfile({
          name: name.trim(),
          brand: selection.brand,
          model: selection.model,
          year: selection.year,
          definitionId: selection.definitionId,
          fromCatalog: selection.fromCatalog,
          mods,
        });
    onSave(profile, alsoConnect);
  };

  return (
    <div className="wizard" ref={headingRef}>
      {step === "bike" && (
        <StepBike
          catalog={catalog}
          definitions={definitions}
          value={selection}
          onChange={setSelection}
          onNext={() => setStep("mods")}
        />
      )}
      {step === "mods" && (
        <StepMods
          value={mods}
          onChange={setMods}
          onNext={() => setStep("summary")}
          onBack={() => setStep("bike")}
        />
      )}
      {step === "summary" && (
        <StepSummary
          selection={selection}
          mods={mods}
          name={name}
          onNameChange={setName}
          definitions={definitions}
          guidance={guidance}
          onBack={() => setStep("mods")}
          onSave={save}
          busy={busy}
        />
      )}
      <p className="muted small">
        <button className="btn btn-small" onClick={onCancel}>
          Cancel
        </button>
      </p>
    </div>
  );
}
