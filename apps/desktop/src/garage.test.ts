// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  describeMods,
  isStock,
  loadGarage,
  recallActiveProfile,
  rememberActiveProfile,
  saveGarage,
  STOCK_MODS,
  BikeProfile,
} from "./garage";

function profile(id: string): BikeProfile {
  return {
    id,
    name: "Test",
    brand: "MV Agusta",
    model: "Brutale 910",
    year: 2006,
    definitionId: "test-ecu",
    fromCatalog: true,
    mods: STOCK_MODS,
    specOverrides: {},
    createdAtMs: 1,
    updatedAtMs: 1,
  };
}

beforeEach(() => localStorage.clear());

describe("garage persistence", () => {
  it("round-trips profiles through the versioned envelope", () => {
    saveGarage([profile("a"), profile("b")]);
    expect(loadGarage().map((p) => p.id)).toEqual(["a", "b"]);
  });

  it("starts empty on absent, corrupt, or unknown-version storage", () => {
    expect(loadGarage()).toEqual([]);
    localStorage.setItem("motodiag-garage", "not json{");
    expect(loadGarage()).toEqual([]);
    localStorage.setItem("motodiag-garage", JSON.stringify({ version: 99, profiles: [1] }));
    expect(loadGarage()).toEqual([]);
  });

  it("remembers and recalls the active profile, dropping dangling ids", () => {
    const profiles = [profile("a")];
    rememberActiveProfile("a");
    expect(recallActiveProfile(profiles)?.id).toBe("a");
    rememberActiveProfile("gone");
    expect(recallActiveProfile(profiles)).toBeNull();
    rememberActiveProfile(null);
    expect(recallActiveProfile(profiles)).toBeNull();
  });
});

describe("mods description", () => {
  it("stock is stock", () => {
    expect(isStock(STOCK_MODS)).toBe(true);
    expect(describeMods(STOCK_MODS)).toBe("stock");
  });

  it("summarizes a modified setup in one line", () => {
    expect(
      describeMods({
        exhaust: "slip-on-open",
        exhaustMaterial: "titanium",
        eprom: "dealer-eprom",
        airFilter: "high-flow",
        otherNotes: "",
      }),
    ).toBe("open slip-ons (titanium) · dedicated EPROM · high-flow air filter");
  });
});
