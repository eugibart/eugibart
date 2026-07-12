# Research intake

How owner-community knowledge gets from a source into MotoDiag's shipped data,
and — just as importantly — what does **not** get committed.

## The rule in one line

**Raw source material stays out of git. Only extracted, attributed facts
(in the definition TOMLs) and the provenance ledger (`SOURCES.md`) are
committed.**

## Why

MotoDiag ships community-sourced figures under a strict *cite-or-cut* policy:
every non-factory number carries a `source` + `source_url` so a user can follow
it to the thread it came from, and anything that can't be attributed is dropped.
This directory is where the raw material lands while it's being turned into
those cited entries.

Two hard limits, because the app is intended to be sold:

1. **Forum posts, not manuals.** Owner posts on a public forum, cited to the
   thread URL, are fair to extract and attribute — the same model the rest of
   the app already uses. Official manufacturer workshop/engine manuals are
   **copyrighted**: use them only as a private reference to *verify* a number
   you then cite to a forum post or to the manual as a factual spec — never
   copy their text, tables, or diagrams into the app or this repo.
2. **Don't redistribute the source.** Bulk forum text and manual scans are not
   committed (that would republish someone else's copyrighted content inside a
   product). Extract the individual fact, cite where it came from, and let the
   raw file stay local.

## Layout

```
research/
  README.md          # this file (committed)
  SOURCES.md         # provenance ledger — thread/URL/date/what it backed (committed)
  revlimiter/        # raw provided files (GITIGNORED — never committed)
  <other-source>/raw/  # any nested raw/ dir is gitignored too
```

`research/revlimiter/` and any `**/raw/` directory are in `.gitignore`. They
exist only in a working checkout; the durable outputs are the TOML edits and
`SOURCES.md`.

## Process

1. Drop the provided file(s) into `research/revlimiter/`.
2. Read them; extract only concrete, attributable facts (idle/TPS/battery/CO
   figures, service-procedure steps and preconditions). Skip anything from a
   manual PDF and anything that can't be tied to a specific post.
3. Apply to the relevant `definitions/**/*.toml`:
   - **Specs** — set `min`/`max`/`target`/`condition` and `source` +
     `source_url` on the channel's `[channels.spec]`.
   - **Procedures** — refine the routine's `procedure`/`preconditions` and set
     the routine-level `source` + `source_url`.
   - Keep `verified = false`. A forum citation makes a value
     *community-sourced and attributed* — it is **not** the same as confirming
     it on a real bike (that's milestones M1/M2/M4). Only a real-bike capture
     flips `verified`.
   - If a forum figure contradicts what's shipped, correct it and record the
     change — honesty over prior guesses.
4. Add a row to `SOURCES.md` for each fact used.
