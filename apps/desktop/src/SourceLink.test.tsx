import { describe, expect, it } from "vitest";
import SourceLink from "./SourceLink";

// The component's defense-in-depth guard: anything that isn't https renders
// nothing at all (the capability scope is the real gate; this stops a
// data-path mistake from ever reaching it).
describe("SourceLink https guard", () => {
  it("renders nothing for non-https URLs", () => {
    expect(SourceLink({ site: "x", url: "http://example.com" })).toBeNull();
    expect(SourceLink({ site: "x", url: "javascript:alert(1)" })).toBeNull();
    expect(SourceLink({ site: "x", url: "file:///etc/passwd" })).toBeNull();
  });

  it("renders an element for https URLs", () => {
    expect(SourceLink({ site: "mvagusta.net", url: "https://www.mvagusta.net/threads/1/" })).not.toBeNull();
  });
});
