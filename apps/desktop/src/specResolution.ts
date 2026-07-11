// Single source of truth for "what reference range applies to this channel,
// on this bike, right now" — with provenance, so the UI and the health
// report can always say WHERE a number came from.
//
// Precedence: user override ("your tuner's figure") > community-adjusted
// (mods.toml, unverified by construction) > stock (the definition's spec).
// The health report does not re-resolve: the frontend passes this module's
// output in, so the logic can never fork.
import {
  ChannelSpecInfo,
  DefinitionInfo,
  ModGuidanceInfo,
  ModRequires,
} from "./ipc";
import { BikeMods, BikeProfile } from "./garage";

export type SpecSource = "stock" | "community-adjusted" | "user-override";

export interface ResolvedSpec {
  min: number | null;
  max: number | null;
  target: number | null;
  condition: string;
  source: SpecSource;
  /** Only a stock spec can be verified; adjusted/override never are. */
  verified: boolean;
  /** Short provenance label for badges and the report's Source column. */
  label: string;
  /** Rationale that came with a community adjustment, if any. */
  note: string | null;
}

const SOURCE_LABELS: Record<SpecSource, string> = {
  stock: "stock reference",
  "community-adjusted": "community reference — unverified",
  "user-override": "your target",
};

/** An entry applies iff, for every requires-field present, the profile's
 *  value is in the listed set. Exhaust material is not matchable. */
export function modsMatch(requires: ModRequires, mods: BikeMods): boolean {
  if (requires.exhaust && !requires.exhaust.includes(mods.exhaust)) return false;
  if (requires.eprom && !requires.eprom.includes(mods.eprom)) return false;
  if (requires.air_filter && !requires.air_filter.includes(mods.airFilter)) return false;
  return true;
}

/** Resolve every channel of `def` for `profile` (null profile = stock view). */
export function resolveSpecs(
  def: DefinitionInfo,
  guidance: ModGuidanceInfo | null,
  profile: BikeProfile | null,
): Record<string, ResolvedSpec> {
  const out: Record<string, ResolvedSpec> = {};

  // First matching adjustment per channel wins (mods.toml is ordered
  // most-specific first and documents this).
  const adjusted: Record<string, { spec: ChannelSpecInfo; note: string | null }> = {};
  if (guidance && profile) {
    for (const adj of guidance.adjustments) {
      if (adj.definition_id !== def.id) continue;
      if (!modsMatch(adj.requires, profile.mods)) continue;
      for (const over of adj.channel_overrides) {
        if (!(over.channel in adjusted)) {
          adjusted[over.channel] = {
            spec: { min: over.min, max: over.max, target: over.target, condition: over.condition },
            note: over.note,
          };
        }
      }
    }
  }

  for (const channel of def.channels) {
    const override = profile?.specOverrides[channel.key];
    if (override && (override.min !== null || override.max !== null || override.target !== null)) {
      out[channel.key] = {
        min: override.min,
        max: override.max,
        target: override.target,
        condition: override.note.trim() || "owner-set target",
        source: "user-override",
        verified: false,
        label: SOURCE_LABELS["user-override"],
        note: null,
      };
      continue;
    }
    const adj = adjusted[channel.key];
    if (adj) {
      out[channel.key] = {
        ...adj.spec,
        source: "community-adjusted",
        verified: false,
        label: SOURCE_LABELS["community-adjusted"],
        note: adj.note,
      };
      continue;
    }
    if (channel.spec) {
      out[channel.key] = {
        ...channel.spec,
        source: "stock",
        verified: channel.verified,
        label: SOURCE_LABELS.stock,
        note: null,
      };
    }
  }

  return out;
}

/** Mirrors Rust ChannelSpec::in_range: null when there's nothing to check. */
export function inRange(spec: ResolvedSpec, value: number): boolean | null {
  if (spec.min === null && spec.max === null) return null;
  const aboveMin = spec.min === null || value >= spec.min;
  const belowMax = spec.max === null || value <= spec.max;
  return aboveMin && belowMax;
}

export function applicableProcedureNotes(
  defId: string,
  guidance: ModGuidanceInfo | null,
  profile: BikeProfile | null,
  routineKey: string,
): string[] {
  if (!guidance || !profile) return [];
  return guidance.procedure_notes
    .filter(
      (n) =>
        n.definition_id === defId &&
        n.routine === routineKey &&
        modsMatch(n.requires, profile.mods),
    )
    .map((n) => n.note);
}

export function applicableDtcNotes(
  defId: string,
  guidance: ModGuidanceInfo | null,
  profile: BikeProfile | null,
  code: number,
): { cause: string; check: string }[] {
  if (!guidance || !profile) return [];
  return guidance.dtc_notes
    .filter(
      (n) => n.definition_id === defId && n.code === code && modsMatch(n.requires, profile.mods),
    )
    .map((n) => ({ cause: n.cause, check: n.check }));
}
