import { describe, expect, it } from "vitest";
import { ZONE_COORDS, ZONE_LABELS } from "./BikeDiagram";
import { AccessZone, BodyStyle } from "./ipc";

// The Rust schema's AccessZone enum, mirrored. If a zone is added there,
// this list (and the ipc.ts union) must grow with it — these tests then
// force the diagram maps to cover it for BOTH silhouettes.
const ALL_ZONES: AccessZone[] = [
  "under-seat",
  "under-tank-left",
  "under-tank-right",
  "under-tank-center",
  "tail-section",
  "dash-area",
  "side-panel-left",
  "side-panel-right",
];
const ALL_STYLES: BodyStyle[] = ["naked", "faired"];

describe("BikeDiagram zone maps", () => {
  it("covers every zone for every silhouette, inside the viewBox", () => {
    for (const style of ALL_STYLES) {
      for (const zone of ALL_ZONES) {
        const c = ZONE_COORDS[style][zone];
        expect(c, `${style}/${zone}`).toBeDefined();
        expect(c.x).toBeGreaterThan(0);
        expect(c.x).toBeLessThan(400);
        expect(c.y).toBeGreaterThan(0);
        expect(c.y).toBeLessThan(220);
      }
    }
  });

  it("labels every zone in plain language", () => {
    for (const zone of ALL_ZONES) {
      expect(ZONE_LABELS[zone]).toBeTruthy();
      expect(ZONE_LABELS[zone]).not.toMatch(/-/); // human words, not the enum
    }
  });
});
