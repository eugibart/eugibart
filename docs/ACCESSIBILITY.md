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
  eyeballed, **in both themes**. Dark weakest pairs: faint text `#8a90a0` on
  raised surface = 4.51:1; primary button white on `#d92f27` = 4.79:1. Light
  weakest pairs: muted ink `#57606c` on raised = 6.0:1; status text
  4.8–6.1:1; badge text over its status tint 5.1–5.9:1. The light theme is a
  full token override, not a programmatic inversion — every value re-chosen
  and re-verified. Placeholder text uses the muted color, not faint.
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
  re-orders with CSS. The bike wizard moves focus to each new step's heading
  (`<h3 tabIndex={-1}>`) when the step changes, so a screen-reader user gets
  "Step 2 of 3 — Stock or modified?" announced without hunting for it — the
  same pattern used for step 1/2/3.
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
  actions state their consequence and exactly what to type/press. The bike
  wizard's mod choices are real `<fieldset>`/`<legend>` radio groups (not
  a styled div), and every spec-override input carries a visually-hidden
  label naming both the channel and the unit (e.g. "Engine speed minimum
  (rpm)") so a screen reader never announces a bare "min" three times per
  channel.

### Robust

- **4.1.2 Name, role, value** — Toggles expose `aria-pressed`
  (shortcuts, pause); the active tab exposes `aria-current`; disabled states
  use the real `disabled` attribute with a `title` explaining why. The
  Advanced ECU-definition picker is a disclosure button with
  `aria-expanded`/`aria-controls`; the Dashboard's "Your targets" editor uses
  a native `<details>/<summary>` — both get correct expanded/collapsed
  semantics and keyboard toggling for free, no custom ARIA needed.
- **4.1.3 Status messages** — Success/notice boxes use `role="status"`
  (routine results, log/report paths, "no fault codes", troubleshooter
  results, pause notices, the top-bar connection status); errors use
  `role="alert"`. Rapid-fire live data is deliberately **not** in a live
  region — announcing a 500 ms feed would be noise; the pause control plus
  focusable sparklines are the accessible path to those values.

## How this was verified

- **Automated**: axe-core (rule tags `wcag2a`, `wcag2aa`, `wcag21a`,
  `wcag21aa`, `wcag22aa`) run via Playwright against every app state, **in
  each theme** (38 audited states total): the base walkthrough — connect
  (empty garage), connect+troubleshooter, Advanced expanded, dashboard (live
  and paused), fault codes, service (read-only and confirm), sync with gauge
  connected, and logging — plus the garage/wizard walkthrough — a
  garage-populated connect screen, all three wizard steps (including the
  "we don't have this bike yet" gap-note state), a dashboard showing a
  community-adjusted range with the overrides editor open, and a fault-code
  card / service routine carrying a mods-based note. **0 violations.**
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

## Citations

Community-sourced figures and mod notes carry a clickable "Source: {site} ↗"
control. Accessibility notes: it is a real `<button>` (it invokes the scoped
system-browser opener, not webview navigation) with a visually-hidden
"(opens in your browser)" suffix so screen-reader users know the context
switch is coming; its color is a dedicated `--link` token computed per theme
for **text-grade 4.5:1** on every surface it appears on — including the
alpha-composited amber mod-note tint (dark `#6db1f7`: 5.5–7.2:1; light
`#2565b9`: 5.2–5.8:1). The axe pass caught the original attempt to reuse the
graphics-grade viz hue here (3.4:1 on the tint) — that's why the separate
token exists.

## Theming

The app ships dark and light themes: a toggle in the top bar, persisted in
`localStorage` (`motodiag-theme`), defaulting to the OS `prefers-color-scheme`
until the user picks one. Implementation notes that matter for accessibility:

- The theme is applied as `data-theme` on `<html>` **before first paint**
  (inline script in `index.html`, mirrored by `initialTheme()` in `App.tsx`)
  so there is no wrong-theme flash.
- `color-scheme` is set per theme so native controls (select popups,
  scrollbars) match.
- Each theme is a complete token set with its own computed contrast — the
  data-viz hue differs per theme (`#3987e5` dark / `#2f74d4` light) because
  each was validated against its own surface (lightness band, chroma floor,
  ≥3:1 vs surface).

## Rules for contributors

1. **Compute contrast, never eyeball it — in both themes.** New color pairs
   must show the ratio in the PR description (4.5:1 text, 3:1 UI/graphics).
   A new color means a new token with a value per theme; never hard-code a
   color that only works on one surface.
2. **No hover-only or color-only information.** Anything a tooltip or hue
   conveys must also exist as text reachable by keyboard.
3. **New polling loops get a pause control** wired to actually stop the timer.
4. **New single-key shortcuts must respect the existing shortcuts toggle.**
5. **Errors → `role="alert"`, outcomes → `role="status"`,** and never put a
   sub-second feed in a live region.
