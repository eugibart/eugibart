import { describe, expect, it } from "vitest";
import { BikeProfile, STOCK_MODS } from "./garage";
import { DefinitionInfo, ModGuidanceInfo } from "./ipc";
import {
  applicableDtcNotes,
  applicableProcedureNotes,
  inRange,
  modsMatch,
  resolveSpecs,
} from "./specResolution";

const def: DefinitionInfo = {
  id: "test-ecu",
  name: "Test ECU",
  manufacturer: "Test",
  models: [],
  bus: "K-line",
  verified: false,
  notes: null,
  channels: [
    {
      key: "rpm",
      name: "Engine speed",
      unit: "rpm",
      verified: true,
      spec: { min: 1100, max: 1300, target: null, condition: "warm idle", source: null, source_url: null },
    },
    {
      key: "batt",
      name: "Battery voltage",
      unit: "V",
      verified: false,
      spec: { min: 13, max: 14.8, target: null, condition: "engine running", source: null, source_url: null },
    },
    { key: "iat", name: "Intake air temperature", unit: "°C", verified: false, spec: null },
  ],
  routines: [],
  charging: null,
};

const guidance: ModGuidanceInfo = {
  adjustments: [
    {
      definition_id: "test-ecu",
      requires: { exhaust: ["slip-on-open", "full-system"], eprom: ["stock"], air_filter: null },
      channel_overrides: [
        {
          channel: "rpm",
          min: 1150,
          max: 1450,
          target: null,
          condition: "warm idle, open exhaust — community reference, unverified",
          note: "idles higher",
          source: "mvagusta.net",
          source_url: "https://www.mvagusta.net/threads/example.1/",
        },
      ],
    },
    {
      // Later entry also matches a modded (open-exhaust) bike: first-in-file
      // must win. Deliberately doesn't match a fully stock profile, so it
      // can't leak into the "stock bike" test case below.
      definition_id: "test-ecu",
      requires: { exhaust: ["slip-on-open", "full-system"], eprom: null, air_filter: null },
      channel_overrides: [
        {
          channel: "rpm",
          min: 1,
          max: 2,
          target: null,
          condition: "should never win",
          note: null,
          source: "mvagusta.net",
          source_url: "https://www.mvagusta.net/threads/example.2/",
        },
      ],
    },
  ],
  procedure_notes: [
    {
      definition_id: "test-ecu",
      routine: "co_trim",
      requires: { exhaust: ["slip-on-open", "full-system"], eprom: null, air_filter: null },
      note: "open exhaust CO caveat",
      source: "mvagusta.net",
      source_url: "https://www.mvagusta.net/threads/example.3/",
    },
  ],
  dtc_notes: [
    {
      definition_id: "test-ecu",
      code: 0x0171,
      requires: { exhaust: ["slip-on-open"], eprom: ["stock"], air_filter: null },
      cause: "lean from open pipes",
      check: "get a proper map",
      source: "ducati.ms",
      source_url: "https://www.ducati.ms/threads/example.4/",
    },
  ],
};

function profileWith(overrides: Partial<BikeProfile>): BikeProfile {
  return {
    id: "p1",
    name: "Test bike",
    brand: "MV Agusta",
    model: "Brutale 910",
    year: 2006,
    definitionId: "test-ecu",
    fromCatalog: true,
    mods: STOCK_MODS,
    specOverrides: {},
    createdAtMs: 0,
    updatedAtMs: 0,
    ...overrides,
  };
}

const moddedMods = {
  ...STOCK_MODS,
  exhaust: "slip-on-open" as const,
};

describe("modsMatch", () => {
  it("matches when every present field's value is in the set", () => {
    expect(
      modsMatch({ exhaust: ["slip-on-open"], eprom: ["stock"], air_filter: null }, moddedMods),
    ).toBe(true);
  });
  it("rejects when any present field misses", () => {
    expect(
      modsMatch({ exhaust: ["full-system"], eprom: null, air_filter: null }, moddedMods),
    ).toBe(false);
  });
  it("matches everything when no fields are present", () => {
    expect(modsMatch({ exhaust: null, eprom: null, air_filter: null }, STOCK_MODS)).toBe(true);
  });
});

describe("resolveSpecs precedence", () => {
  it("stock bike gets stock specs with stock provenance", () => {
    const specs = resolveSpecs(def, guidance, profileWith({}));
    expect(specs.rpm).toMatchObject({ min: 1100, max: 1300, source: "stock", verified: true });
    expect(specs.iat).toBeUndefined();
  });

  it("no profile means stock view", () => {
    const specs = resolveSpecs(def, guidance, null);
    expect(specs.rpm.source).toBe("stock");
  });

  it("matching mods swap in the community adjustment, first-in-file wins, never verified", () => {
    const specs = resolveSpecs(def, guidance, profileWith({ mods: moddedMods }));
    expect(specs.rpm).toMatchObject({
      min: 1150,
      max: 1450,
      source: "community-adjusted",
      verified: false,
      note: "idles higher",
    });
    // Non-adjusted channels keep stock.
    expect(specs.batt.source).toBe("stock");
  });

  it("user override beats both, and is never verified", () => {
    const specs = resolveSpecs(
      def,
      guidance,
      profileWith({
        mods: moddedMods,
        specOverrides: { rpm: { min: 1250, max: 1350, target: null, note: "tuner's figure" } },
      }),
    );
    expect(specs.rpm).toMatchObject({
      min: 1250,
      max: 1350,
      source: "user-override",
      verified: false,
      condition: "tuner's figure",
    });
  });

  it("an all-null override is ignored rather than blanking the spec", () => {
    const specs = resolveSpecs(
      def,
      guidance,
      profileWith({
        specOverrides: { rpm: { min: null, max: null, target: null, note: "" } },
      }),
    );
    expect(specs.rpm.source).toBe("stock");
  });
});

describe("inRange", () => {
  const spec = {
    min: 10,
    max: 20,
    target: null,
    condition: "c",
    source: "stock" as const,
    verified: true,
    label: "stock reference",
    note: null,
    citationSite: null,
    citationUrl: null,
  };
  it("classifies inside/outside and honors open bounds", () => {
    expect(inRange(spec, 15)).toBe(true);
    expect(inRange(spec, 25)).toBe(false);
    expect(inRange({ ...spec, max: null }, 1000)).toBe(true);
    expect(inRange({ ...spec, min: null, max: null }, 15)).toBeNull();
  });
});

describe("guidance note selection", () => {
  it("procedure notes apply only to matching mods and routine", () => {
    expect(
      applicableProcedureNotes("test-ecu", guidance, profileWith({ mods: moddedMods }), "co_trim"),
    ).toEqual([{ note: "open exhaust CO caveat", source: "mvagusta.net", sourceUrl: "https://www.mvagusta.net/threads/example.3/" }]);
    expect(
      applicableProcedureNotes("test-ecu", guidance, profileWith({}), "co_trim"),
    ).toEqual([]);
    expect(
      applicableProcedureNotes("test-ecu", guidance, profileWith({ mods: moddedMods }), "tps"),
    ).toEqual([]);
  });

  it("dtc notes match by code and mods; none without a profile", () => {
    expect(
      applicableDtcNotes("test-ecu", guidance, profileWith({ mods: moddedMods }), 0x0171),
    ).toEqual([{ cause: "lean from open pipes", check: "get a proper map", source: "ducati.ms", sourceUrl: "https://www.ducati.ms/threads/example.4/" }]);
    expect(applicableDtcNotes("test-ecu", guidance, null, 0x0171)).toEqual([]);
  });
});
