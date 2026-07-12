import { describe, expect, it } from "vitest";
import { splitPlaceholder } from "./dtcDisplay";

describe("splitPlaceholder", () => {
  it("strips the honest-data suffix and flags it", () => {
    expect(splitPlaceholder("System too lean, bank 1 (placeholder)")).toEqual({
      text: "System too lean, bank 1",
      placeholder: true,
    });
  });

  it("leaves ordinary descriptions untouched", () => {
    expect(splitPlaceholder("Coolant temperature sensor — open circuit")).toEqual({
      text: "Coolant temperature sensor — open circuit",
      placeholder: false,
    });
  });

  it("does not eat parentheses that are part of the description", () => {
    expect(splitPlaceholder("TPS (throttle position sensor) implausible")).toEqual({
      text: "TPS (throttle position sensor) implausible",
      placeholder: false,
    });
  });
});
