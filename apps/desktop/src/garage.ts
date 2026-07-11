// The garage: saved bike profiles ("tell me about your bike" wizard output),
// persisted in localStorage under a versioned envelope. Follows the same
// client-side persistence pattern as the theme and shortcuts settings.
import { useCallback, useSyncExternalStore } from "react";
import { AirFilterMod, EpromMod, ExhaustMaterial, ExhaustMod } from "./ipc";

export interface BikeMods {
  exhaust: ExhaustMod;
  /** Informational only (never affects guidance): sound and weight, not fueling. */
  exhaustMaterial: ExhaustMaterial | null;
  eprom: EpromMod;
  airFilter: AirFilterMod;
  /** Free text, displayed and reported but never matched against guidance. */
  otherNotes: string;
}

/** A per-channel target the owner (or their tuner) typed in. Wins over both
 *  stock and community-adjusted specs. */
export interface UserSpecOverride {
  min: number | null;
  max: number | null;
  target: number | null;
  /** Where the figure came from, e.g. "tuner's dyno sheet, May 2025". */
  note: string;
}

export interface BikeProfile {
  id: string;
  name: string;
  brand: string;
  model: string;
  year: number | null;
  definitionId: string;
  /** False when the definition was hand-picked via the Advanced section. */
  fromCatalog: boolean;
  mods: BikeMods;
  specOverrides: Record<string, UserSpecOverride>;
  createdAtMs: number;
  updatedAtMs: number;
}

export const STOCK_MODS: BikeMods = {
  exhaust: "stock",
  exhaustMaterial: null,
  eprom: "stock",
  airFilter: "stock",
  otherNotes: "",
};

export function isStock(mods: BikeMods): boolean {
  return mods.exhaust === "stock" && mods.eprom === "stock" && mods.airFilter === "stock";
}

/** One-line human summary, e.g. "open slip-ons (titanium) · dealer EPROM". */
export function describeMods(mods: BikeMods): string {
  if (isStock(mods)) return "stock";
  const parts: string[] = [];
  if (mods.exhaust !== "stock") {
    const kind = mods.exhaust === "slip-on-open" ? "open slip-ons" : "full exhaust system";
    parts.push(mods.exhaustMaterial ? `${kind} (${mods.exhaustMaterial})` : kind);
  }
  if (mods.eprom !== "stock") {
    parts.push(mods.eprom === "dealer-eprom" ? "dedicated EPROM" : "custom map");
  }
  if (mods.airFilter !== "stock") parts.push("high-flow air filter");
  if (mods.otherNotes.trim()) parts.push(mods.otherNotes.trim());
  return parts.join(" · ");
}

const GARAGE_KEY = "motodiag-garage";
const ACTIVE_PROFILE_KEY = "motodiag-active-profile";
const GARAGE_VERSION = 1;

interface GarageEnvelope {
  version: number;
  profiles: BikeProfile[];
}

export function loadGarage(): BikeProfile[] {
  try {
    const raw = localStorage.getItem(GARAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as GarageEnvelope;
    // Unknown future version: don't guess at its shape, start empty rather
    // than corrupt it. (Saving will rewrite as the current version.)
    if (parsed.version !== GARAGE_VERSION || !Array.isArray(parsed.profiles)) return [];
    return parsed.profiles;
  } catch {
    return [];
  }
}

export function saveGarage(profiles: BikeProfile[]): void {
  localStorage.setItem(GARAGE_KEY, JSON.stringify({ version: GARAGE_VERSION, profiles }));
}

export function rememberActiveProfile(id: string | null): void {
  if (id === null) localStorage.removeItem(ACTIVE_PROFILE_KEY);
  else localStorage.setItem(ACTIVE_PROFILE_KEY, id);
}

export function recallActiveProfile(profiles: BikeProfile[]): BikeProfile | null {
  const id = localStorage.getItem(ACTIVE_PROFILE_KEY);
  return profiles.find((p) => p.id === id) ?? null;
}

// A tiny external store so every component sees garage edits immediately
// (useSyncExternalStore keeps this simple and tear-free).
let cache: BikeProfile[] | null = null;
const listeners = new Set<() => void>();

function readGarage(): BikeProfile[] {
  if (cache === null) cache = loadGarage();
  return cache;
}

function writeGarage(profiles: BikeProfile[]): void {
  cache = profiles;
  saveGarage(profiles);
  for (const l of listeners) l();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useGarage() {
  const profiles = useSyncExternalStore(subscribe, readGarage);

  const addProfile = useCallback((profile: BikeProfile) => {
    writeGarage([...readGarage(), profile]);
  }, []);

  const updateProfile = useCallback((profile: BikeProfile) => {
    writeGarage(
      readGarage().map((p) => (p.id === profile.id ? { ...profile, updatedAtMs: Date.now() } : p)),
    );
  }, []);

  const removeProfile = useCallback((id: string) => {
    writeGarage(readGarage().filter((p) => p.id !== id));
  }, []);

  return { profiles, addProfile, updateProfile, removeProfile };
}

export function newProfile(
  fields: Omit<BikeProfile, "id" | "specOverrides" | "createdAtMs" | "updatedAtMs">,
): BikeProfile {
  return {
    ...fields,
    id: crypto.randomUUID(),
    specOverrides: {},
    createdAtMs: Date.now(),
    updatedAtMs: Date.now(),
  };
}
