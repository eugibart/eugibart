# MotoDiag roadmap

Future requirements, in dependency order, plus an honest inventory of today's
gaps. Companion to [MOTODIAG.md](MOTODIAG.md) (what the app is) — this file is
*what's missing and what comes next*. The original M0–M5 milestone list lives
in MOTODIAG.md's Roadmap section; this supersedes it in scope.

**The bar for "done" here:** MotoDiag's honesty model means a feature isn't
finished when the code merges — it's finished when its numbers are confirmed
against real hardware (`verified = true`), its citation points somewhere real,
and the UI says so. Several phases below exist purely to reach that bar.

---

## Current gaps (flagged)

### Trust & data — the big one

- **Nothing is hardware-verified yet.** All **31** `verified` flags across the
  six definition files are `false`; `verified = true` appears nowhere in the
  repo. Every spec, DTC table, local identifier, and identification byte is an
  educated, cited-where-possible placeholder pending M1/M2/M4 on a real bike.
- **Placeholder ECU address** `0x10` in `definitions/mv/5sm-brutale-910.toml`
  — the single most load-bearing unverified byte in the project.
- **Six "(placeholder)" DTC descriptions** still ship (2× `5sm-brutale-910`,
  2× `iaw-5am`, 1× `iaw-59m`, 1× `mc3-multistrada-can`). The UI badges them
  UNVERIFIED honestly, but they're gaps nonetheless.
- **`docs/MV-PINOUT.md` is an empty placeholder** — "to be filled in during
  milestone M1." A tool that tells owners to probe connectors should ship a
  verified pinout before first field use.
- **Definition stubs**: `definitions/mv/7bm-f4-312r.toml` is a minimal
  rpm-only stub (wrong-ECU-family warning included);
  `definitions/ducati/mc3-multistrada-can.toml` is a schema demonstration,
  not a working definition.
- **~15 catalog entries carry a `gap_note`** for ECU generations the app
  can't talk to at all (Marelli 1.6M/15M/P8, Siemens/Continental M3C,
  CAN-era 848/1098/1198). Documented, not hidden — but each is a coverage gap.
- **Held-back forum knowledge**: rich Revlimiter.it content (modified-exhaust
  idle/CO targets, decel-popping guidance, collector-measurement note, a full
  vacuum-balance procedure) is extracted and ready but **cannot ship until the
  original thread URLs are provided** — the schema structurally requires a
  real citation. See `research/SOURCES.md`.

### Reachability — built but unwired

- **The whole CAN/UDS stack is unreachable from the app.** `protocol-can`
  (ISO-TP, UDS session, SLCAN + ELM327 backends) is built and tested end-to-end
  against a bench simulator, but is not wired into `app-core::DiagSession` or
  the desktop UI. A CAN-era Ducati owner can't use MotoDiag today at all.

### Distribution — no path to users

- **Bundling is off**: `apps/desktop/src-tauri/tauri.conf.json` has
  `"bundle": { "active": false }` — no installers are produced, ever.
- **One single PNG icon** (`icons/icon.png`); a release needs proper
  `.icns`/`.ico`/multi-resolution sets.
- **No code signing, no macOS notarization, no auto-updater** — nothing of the
  kind is configured. For a Mac-first paid app this is table stakes: unsigned
  apps trip Gatekeeper.
- **CI never builds a release.** `.github/workflows/ci.yml` runs the full test
  and audit matrix (good) but there is no release/publish workflow and no
  artifact a user could download.

### Product completeness

- **No Settings screen** — theme and shortcuts live in the topbar; there's no
  home for preferences (units, default port, language, updates).
- **Garage profiles are trapped in `localStorage`** — no export/import; a
  reinstall or new machine loses every bike, mod combo, and spec override.
- **No localization.** All UI text is inline English in JSX; there's no i18n
  layer. Ironic for an app whose best knowledge sources — and likely a large
  slice of its buyers — are Italian.
- **High-contrast / forced-colors mode not yet styled**
  (`docs/ACCESSIBILITY.md:148`, tracked as future work).
- **Researched backlog from MOTODIAG.md still open**: log charts and a
  community definition-exchange flow.

### Engineering debt

- **Frontend tests cover only pure helpers** (8 test files: garage, snapshots,
  specResolution, dtcDisplay, gaugeStatus, BikeDiagram, SourceLink,
  HelpScreen). Every functional screen, the bike wizard, and the panels
  (Compare, ChargingTest, HowTo, SpecOverrides) have no automated tests in the
  repo — the Playwright behavior/axe harness that verified them lives outside
  it and dies with each dev session.
- **The Tauri command layer** (`apps/desktop/src-tauri/src/commands.rs`) is
  only `cargo check`ed in CI, and its ten `lock().unwrap()` mutex sites would
  panic the command thread on lock poisoning instead of degrading gracefully.

---

## Future requirements, phased

Ordered by dependency, not glamour: Phase 1 gates the product's core promise,
Phase 2 gates selling it at all.

### Phase 1 — Prove it on metal (M1 / M2 / M4)

*Everything the app claims is provisional until this happens. Needs: the
physical Brutale, the FTDI KKL cable + 3-pin adapter
([docs/HARDWARE.md](docs/HARDWARE.md)), a multimeter; for M4, a Windows box
running JPDiag.*

1. **Verify the diagnostic connector pinout** on the real bike; fill in
   `docs/MV-PINOUT.md` with photos-free, measured facts.
2. **First real K-line connection + ECU identification** — resolve the `0x10`
   address placeholder; capture the whole session with `--trace` so it becomes
   a permanent replay fixture in `fixtures/`.
3. **Run `motodiag-discover` against the real ECU** — map the actual live-data
   local identifiers, identification bytes, and DTC read variants.
4. **Validate DTCs + live data (M2)** — confirm each channel's scaling against
   the dash/known states; flip each confirmed spec to `verified = true`
   (per-value, never wholesale); replace confirmed placeholder DTC texts.
5. **Sniff JPDiag for service functions (M4)** — TPS reset first, then CO
   trim and actuator tests; encode as routines with verified request bytes.

**Acceptance:** first definition file with `verified = true` entries; a
committed real-bike wire-trace fixture replaying green in CI; MV-PINOUT.md
filled; the Help screen's "first connection" story updated from theory to
fact.

### Phase 2 — Make it installable (release pipeline)

*Independent of Phase 1; do in parallel. Gates any sale or beta.*

1. Enable bundling (`bundle.active: true`), define targets (dmg/app for
   macOS first), generate the full icon set from a master asset.
2. macOS **code signing + notarization** (Developer ID) wired into a release
   build; document the secrets setup.
3. **Auto-updater** (Tauri updater plugin) with signed update manifests —
   a capability addition, so re-run the `cargo check`/capability review.
4. **Release CI workflow**: tag-triggered `tauri build` matrix, artifacts
   attached to a GitHub release; version bump + changelog convention.
5. Decide licensing/purchase mechanics (out of repo scope, but the updater
   and versioning choices depend on it).

**Acceptance:** a tagged release produces a signed, notarized .dmg a stranger
can download, open without Gatekeeper overrides, and auto-update.

### Phase 3 — Data depth (the moat)

*The sellable difference is the definitions, not the transport code.*

1. **Flesh out `7bm-f4-312r.toml`** from stub to full definition (channels,
   specs, DTC table, routines, access guides) — the F4 owner experience today
   is one rpm gauge.
2. **Ship the held-back Revlimiter knowledge** the moment thread URLs arrive;
   continue the `research/SOURCES.md` intake loop for new material.
3. **Retire the six placeholder DTC texts** as real descriptions are sourced.
4. **Brutale silhouette measured pass** — same dimension-true treatment the F4
   diagram got, once a side-on reference is provided.
5. **Widen model coverage** guided by `catalog.toml` gap_notes — nearest wins
   first (other 5SM-family MVs, more K-line Ducatis), each new bike = new
   TOML, no new code.

**Acceptance:** no shipped definition is a stub; every community figure cited;
DTC placeholder count zero for covered models.

### Phase 4 — CAN era (unlock modern Ducatis)

1. **Wire `protocol-can` into `app-core::DiagSession`** behind the existing
   session abstraction; surface transport choice (K-line / SLCAN / ELM327) in
   the Connect wizard.
2. Replace `mc3-multistrada-can.toml`'s demo content with a real definition
   (community DDA/MelcoDiag knowledge, cited).
3. **Hardware validation** with an OBDLink SX/EX and/or CANable 2.0 —
   the CAN twin of Phase 1, with the same trace-to-fixture loop.

**Acceptance:** a CAN-era Ducati connects through the same wizard, with the
same honesty model, and a real CAN wire-trace fixture replays in CI.

### Phase 5 — Product polish

1. **Settings screen** — consolidate theme, shortcuts, default port, language,
   update channel; new tab, same a11y bar (axe 0 violations both themes).
2. **Garage export/import** — JSON file reusing the versioned envelope in
   `apps/desktop/src/garage.ts`; makes profiles portable and shareable
   (a mate's identical bike = one import).
3. **Italian localization** — extract inline strings to an i18n layer first
   (the real work), then translate; Italian is the natural first locale given
   the knowledge base and market.
4. **Log charts** — plot CSV/live history over time (the researched-backlog
   item); sparklines exist, real charts don't.
5. **Forced-colors / high-contrast styling** to close the ACCESSIBILITY.md
   caveat.
6. **Community loops**: definition-exchange flow (share/import definition
   improvements with provenance) and a contribute-a-citation flow (submit a
   thread URL for an uncited local observation).

### Phase 6 — Hardening

1. **Promote the UI behavior harness into the repo** (`apps/desktop/e2e/`):
   the Playwright + axe patterns already proven in development (mock IPC
   layer, per-screen behavior checks, dual-theme accessibility sweep) should
   run in CI, not die with a dev machine.
2. **Screen-level component tests** for the wizard, Dashboard spec rendering,
   and DTC/Service mod-note weaving — the logic-heavy interactive paths.
3. **Command-layer tests** for `commands.rs`, and replace the ten
   `lock().unwrap()` sites with poison-tolerant handling so one panicked
   thread can't wedge every subsequent command.

---

## Non-goals (permanent, restated)

These are design guarantees, not gaps — listed so nobody "fixes" them:

- **ECU map/flash read-write stays structurally impossible** — the definition
  validator rejects memory/flash service IDs on both buses
  ([docs/SAFETY.md](docs/SAFETY.md), [docs/PROTOCOL.md](docs/PROTOCOL.md)).
- **Official manufacturer manuals are never extracted into or shipped with
  the app** — private verify-only reference (`research/README.md`).
- **No uncited community claims, no fabricated URLs** — the schema requires a
  source on every community claim; anything that can't be cited gets cut or
  shipped as clearly-labeled prose corroboration only.
- **`verified = true` only ever comes from real hardware** — never from
  agreement between forum posts.
