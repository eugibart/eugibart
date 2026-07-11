import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import HelpScreen from "./HelpScreen";

// The table of contents and the section headings are maintained by hand in
// the same file — these tests are the tripwire for the two drifting apart
// (a ToC link pointing at a missing id is a silent dead anchor).
describe("HelpScreen anchors", () => {
  const html = renderToStaticMarkup(<HelpScreen />);
  const tocTargets = [...html.matchAll(/href="#(help-[a-z-]+)"/g)].map((m) => m[1]);

  it("has a table of contents", () => {
    expect(tocTargets.length).toBeGreaterThanOrEqual(10);
  });

  it("every ToC link targets an existing heading that can take focus", () => {
    for (const id of new Set(tocTargets)) {
      // tabIndex={-1} on the target heading is what moves focus on jump
      // (WCAG 2.4.3) — the id alone would only scroll.
      expect(html).toContain(`id="${id}" tabindex="-1"`);
    }
  });

  it("every section heading is reachable from the ToC", () => {
    const headingIds = [...html.matchAll(/<h3 id="(help-[a-z-]+)"/g)].map((m) => m[1]);
    for (const id of headingIds) {
      expect(tocTargets).toContain(id);
    }
  });
});
