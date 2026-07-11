import { DefinitionInfo } from "./ipc";
import { BikeProfile, UserSpecOverride } from "./garage";
import { ResolvedSpec } from "./specResolution";

/**
 * Per-channel target editor: "your tuner's figure". Values here win over
 * both stock and community-adjusted specs (see specResolution.ts) and drive
 * the Dashboard's in-range badges and the health report.
 */
export default function SpecOverridesEditor({
  definition,
  profile,
  resolvedSpecs,
  onChange,
}: {
  definition: DefinitionInfo;
  profile: BikeProfile;
  resolvedSpecs: Record<string, ResolvedSpec>;
  onChange: (profile: BikeProfile) => void;
}) {
  const setOverride = (key: string, patch: Partial<UserSpecOverride>) => {
    const current: UserSpecOverride = profile.specOverrides[key] ?? {
      min: null,
      max: null,
      target: null,
      note: "",
    };
    onChange({
      ...profile,
      specOverrides: { ...profile.specOverrides, [key]: { ...current, ...patch } },
    });
  };

  const clearOverride = (key: string) => {
    const rest = { ...profile.specOverrides };
    delete rest[key];
    onChange({ ...profile, specOverrides: rest });
  };

  const numOrNull = (raw: string) => (raw.trim() === "" ? null : Number(raw));

  return (
    <div className="overrides-editor">
      {definition.channels.map((c) => {
        const ov = profile.specOverrides[c.key];
        const resolved = resolvedSpecs[c.key];
        const unitLabel = c.unit ? ` (${c.unit})` : "";
        return (
          <div className="override-row" key={c.key}>
            <span className="override-channel">{c.name}</span>

            <label className="visually-hidden" htmlFor={`ov-min-${c.key}`}>
              {c.name} minimum{unitLabel}
            </label>
            <input
              id={`ov-min-${c.key}`}
              type="number"
              placeholder={resolved?.min?.toString() ?? "min"}
              value={ov?.min ?? ""}
              onChange={(e) => setOverride(c.key, { min: numOrNull(e.target.value) })}
            />

            <label className="visually-hidden" htmlFor={`ov-max-${c.key}`}>
              {c.name} maximum{unitLabel}
            </label>
            <input
              id={`ov-max-${c.key}`}
              type="number"
              placeholder={resolved?.max?.toString() ?? "max"}
              value={ov?.max ?? ""}
              onChange={(e) => setOverride(c.key, { max: numOrNull(e.target.value) })}
            />

            <label className="visually-hidden" htmlFor={`ov-target-${c.key}`}>
              {c.name} target{unitLabel}
            </label>
            <input
              id={`ov-target-${c.key}`}
              type="number"
              placeholder={resolved?.target?.toString() ?? "target"}
              value={ov?.target ?? ""}
              onChange={(e) => setOverride(c.key, { target: numOrNull(e.target.value) })}
            />

            <label className="visually-hidden" htmlFor={`ov-note-${c.key}`}>
              Where {c.name}'s target came from
            </label>
            <input
              id={`ov-note-${c.key}`}
              placeholder="e.g. tuner's dyno sheet"
              value={ov?.note ?? ""}
              onChange={(e) => setOverride(c.key, { note: e.target.value })}
            />

            {ov && (
              <button
                className="btn btn-small"
                onClick={() => clearOverride(c.key)}
                aria-label={`Clear your ${c.name} target`}
              >
                Clear
              </button>
            )}
          </div>
        );
      })}
    </div>
  );
}
