# Provenance ledger

Every community-sourced fact that made it into a definition file, and where it
came from. This is attribution metadata — not a copy of the source content (see
`README.md`). One row per fact used.

| Date | Source | Thread / URL | Model / ECU | What it backed | In-app citation |
| ---- | ------ | ------------ | ----------- | -------------- | --------------- |
| 2026-07-12 | "Brad The Bike Boy" (bikeboy.org) — "Weber Marelli Throttle Position Sensor Setting Notes and Procedures" | **URL unknown** — received as a PDF (via Eugenio, said to be encountered on Revlimiter.it); the document itself only links out to mvfaq.blogspot.com, not its own host page. Not yet citable as a clickable link — content used as prose-only corroboration pending a stable URL. | mv-5sm-brutale-910 | Corroborates: (a) the 1.6M-vs-5SM physical/software TPS-baseline split already cited to mvfaq.blogspot.com; (b) the 3.5% (3-4%) idle CO target already cited to mvfaq.blogspot.com. Also confirms MV's own procedure checks idle mixture with the radiator fans running (~100°C+), stricter than the usual 65°C bar — new detail, added to `co_trim` preconditions. | `tps_reset`/`co_trim` procedure prose only (no `source`/`source_url` set — see README's cite-or-cut rule) |
| 2026-07-12 | Revlimiter.it forum thread(s) (Italian; pasted by Eugenio into a .docx) — CO / sync / throttle-body / TPS-reset discussion | **URL unknown** — text was pasted without the original thread link(s). Awaiting the actual URL(s) from Eugenio. | mv-5sm-brutale-910 (Brutale 910R/1078, F4 1000S) | Rich, high-value content **not yet applied** pending the thread URL(s), since `mods.toml`'s adjustment/procedure/DTC-note schemas structurally *require* a real `source`/`source_url` (validated, no optional path): (1) modified-exhaust (full-system/titanium) Brutale 910/1078 → idle ~1300 rpm, idle CO ~4.3-4.5% (vs stock ~3.5%); (2) decel popping on modified-exhaust bikes at stock CO = lean, not a fault — raise CO toward 4.3-4.5% instead; (3) CO must be measured at the central collector, not the tailpipe outlet; (4) a detailed step-by-step vacuum-balance procedure (start fully closed, balance cyl 2↔3 first, then 1↔2, then 3↔4, verify 1↔4). One quoted passage (an "Euro3 manual" ignition-advance-correction parameter, ±4°) is explicitly manual-sourced *within* the forum post — excluded per the manuals policy regardless of how it reached us. | none yet — held pending URL(s) |

## Notes on sources already in the shipped data

The definitions currently cite these community sources (added before this
ledger existed; recorded here for completeness):

- **mvfaq.blogspot.com** — F4 1000 idle target, TPS closed-throttle figures
  (`definitions/mv/5sm-brutale-910.toml`).
- **mvagusta.net** — charging/regulator voltage split, EPROM/exhaust notes.
- **ducati.ms**, **ducatimonster.org** — Ducati idle specs, 5.9M/5AM notes,
  reflash-vs-chip culture (`definitions/ducati/*.toml`, `definitions/mods.toml`).
