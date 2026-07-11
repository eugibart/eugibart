# Accessibility (WCAG 2.2 AA)

MotoDiag targets **WCAG 2.2 Level AA**. This page records how each relevant
success criterion is met, so regressions are catchable in review: if a change
touches one of these behaviours, re-check the criterion it implements.

## How the requirements are met

### Perceivable

- **1.1.1 Non-text content** — Decorative glyphs (status dots, ↻, ✓/✗/▲/▼, the
  brand accent bar) are `aria-hidden` or CSS pseudo-elements; the adjacent text
  carries the meaning. Sparklines expose an `aria-label` with sample count and
  latest value, and every plotted value is also available as text (the tile's
  value readout, plus per-sample inspection — see 2.1.1). Sync bars print the
  exact kPa and delta next to each bar.
- **1.3.1 Info and relationships** — All form controls have programmatic
  labels (`<label htmlFor>`; the Service confirmation input has a
  visually-hidden label). Tabs are `<button>`s inside a labelled `<nav>` with
  `aria-current="page"` on the active one. Lists of steps/causes/checks are
  real `<ol>`/`<ul>` elements. The page has one `<h1>` (brand) and a heading
  per panel.
- **1.4.1 Use of color** — In-range/out-of-range gauge status pairs color with
  a glyph *and* text ("in range (…)"/"outside …"). Sync spread pairs color with
  words ("balanced"/"close, keep going"/"unbalanced"). Verified/unverified is a
  text badge, not a color.
- **1.4.3 Contrast (minimum)** — All text/background pairs are computed, not
  eyeballed. Weakest pairs: faint text `#8a90a0` on raised surface = 4.51:1;
  primary button white on `#d92f27` = 4.79:1; muted and body text well above
  7:1. Placeholder text uses the muted color, not faint.
- **1.4.4 Resize text / 1.4.10 Reflow** — Layout is flex/grid with `flex-wrap`
  on the tab bar, connection status, form rows, and sync bars; wide content
  scrolls in its own container. No information is lost at 400% zoom /
  320 px-equivalent width.
- **1.4.11 Non-text contrast** — Input/button borders use `--border-ui`
  `#767f8f` (≥3:1 against every surface it sits on). The focus indicator and
  sparkline series hue (`#3987e5`) also clear 3:1 against their backgrounds.
- **1.4.13 Content on hover or focus** — The sparkline tooltip is
  **dismissable** (Escape closes it without moving the pointer, via a global
  listener while it is open), **hoverable** in effect (it never covers the
  pointer target area — it sits above the plot), and **persistent** (stays
  until pointer leave, blur, or Escape). Button `title` hints are supplementary
  only; the same information is always available elsewhere in text.

### Operable

- **2.1.1 Keyboard** — Everything hover does, keys do: sparklines are focusable
  (`tabIndex=0`) and Left/Right/Home/End step through samples, echoing the
  inspected value to a polite live region. All actions are native buttons,
  selects, and inputs.
- **2.1.2 No keyboard trap** — No focus traps; confirmation boxes are inline
  content (not modal) and Escape additionally cancels them.
- **2.1.4 Character key shortcuts** — The 1–6 tab shortcuts can be **turned
  off** with the "Shortcuts: on/off" toggle in the top bar (`aria-pressed`,
  persisted in `localStorage`). They are also suppressed while typing in any
  field and when a modifier key is held.
- **2.2.2 Pause, stop, hide** — Both auto-updating screens have a
  **Pause updates / Resume updates** control: the Dashboard (500 ms live-data
  poll) and the Sync screen (300 ms vacuum poll). Pausing stops the poll loop
  entirely — it is not just a visual freeze.
- **2.4.1 Bypass blocks** — A "Skip to content" link is the first tabbable
  element and jumps to `<main id="main" tabIndex={-1}>`.
- **2.4.3 Focus order** — DOM order matches visual order everywhere; nothing
  re-orders with CSS.
- **2.4.7 / 2.4.11 Focus visible, not obscured** — A global
  `:focus-visible` outline (2 px, offset, ≥3:1) applies to every interactive
  element; no sticky/overlay content can cover a focused element.
- **2.5.7 Dragging** — No drag interactions exist.
- **2.5.8 Target size (minimum)** — Buttons and inputs are ≥24 CSS px tall
  including padding; small buttons are the only compact targets and still
  clear 24 px.

### Understandable

- **3.2.1 / 3.2.2 On focus / on input** — Nothing changes context on focus or
  input; connect/run actions are explicit button presses.
- **3.3.1 / 3.3.3 Error identification & suggestion** — Errors appear in a
  `role="alert"` box in plain language; the connection troubleshooter goes
  further and states the failed step plus a concrete suggestion.
- **3.3.2 Labels or instructions** — Every input is labelled; destructive
  actions state their consequence and exactly what to type/press.

### Robust

- **4.1.2 Name, role, value** — Toggles expose `aria-pressed`
  (shortcuts, pause); the active tab exposes `aria-current`; disabled states
  use the real `disabled` attribute with a `title` explaining why.
- **4.1.3 Status messages** — Success/notice boxes use `role="status"`
  (routine results, log/report paths, "no fault codes", troubleshooter
  results, pause notices, the top-bar connection status); errors use
  `role="alert"`. Rapid-fire live data is deliberately **not** in a live
  region — announcing a 500 ms feed would be noise; the pause control plus
  focusable sparklines are the accessible path to those values.

## How this was verified

- **Automated**: axe-core (rule tags `wcag2a`, `wcag2aa`, `wcag21a`,
  `wcag21aa`, `wcag22aa`) run via Playwright against nine app states —
  connect, connect+troubleshooter, dashboard (live and paused), fault codes,
  service (read-only and confirm), sync with gauge connected, and logging —
  using the same mock-IPC harness as the UI screenshots. **0 violations.**
  This pass caught a real defect the hand-computed pass missed: badge text
  sat on a semi-transparent status *tint*, which lightens the effective
  background, dropping ok/danger badge text below 4.5:1 — fixed with the
  dedicated `--ok-on-tint`/`--danger-on-tint` tokens.
- **Behavioural (scripted)**: sparkline focus + Arrow-key sample inspection,
  live-region echo, and Escape dismissal of both keyboard- and
  pointer-opened tooltips.
- **Manual**: contrast ratios for every ink/surface pair computed from the
  token values (WCAG relative-luminance formula), including alpha-composited
  tinted backgrounds.

## Known limits

- The app is a desktop (Tauri/WebKit) app; testing has been with keyboard-only
  operation and static analysis of the accessibility tree. A VoiceOver pass on
  real macOS hardware is still to be done before release and may surface
  wording-level refinements.
- High-contrast / forced-colors mode is not yet explicitly styled
  (`forced-colors` media query) — tracked as future work.

## Rules for contributors

1. **Compute contrast, never eyeball it.** New color pairs must show the ratio
   in the PR description (4.5:1 text, 3:1 UI/graphics).
2. **No hover-only or color-only information.** Anything a tooltip or hue
   conveys must also exist as text reachable by keyboard.
3. **New polling loops get a pause control** wired to actually stop the timer.
4. **New single-key shortcuts must respect the existing shortcuts toggle.**
5. **Errors → `role="alert"`, outcomes → `role="status"`,** and never put a
   sub-second feed in a live region.
