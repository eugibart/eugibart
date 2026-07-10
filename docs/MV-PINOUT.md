# MV Agusta Brutale 910 — diagnostic connector

**STATUS: UNVERIFIED.** Nothing in this file has been confirmed on a real
bike yet. This page exists to be filled in during milestone M1.

## What community reports say

- A 3-pin diagnostic connector under the tank on the right side (Brutale).
- Some bikes: a 4-pin connector under the seat.
- Only three signals are needed for diagnostics: **K-line, +12 V, GND**
  (JPDiag users connect exactly these three wires from a KKL cable).

## Verification procedure (do this before ever connecting the cable)

1. Get the Brutale 910 workshop manual wiring diagram (the F4/Brutale manuals
   circulate on manualslib and owner forums; MV wiring colors are usually
   consistent per model year).
2. Locate the connector(s); photograph them and note the housing type.
3. Key ON, engine off, multimeter:
   - Find the pin at battery voltage (+12 V) referenced to frame ground.
   - Find chassis ground (continuity to battery negative).
   - The remaining pin should be the K-line: expect it to idle near +12 V
     (pulled up inside the ECU) and not be at 0 Ω to ground or battery.
4. Record everything here: connector photos, pin numbering, wire colors,
   measured voltages.
5. Once wired up, run `motodiag-discover` (see docs/PROTOCOL.md) against the
   real bike — no other diagnostic software required. A successful connect
   and ECU identification response confirms the pinout; garbage or no
   response at all is the first sign something's wired wrong.

## Findings (fill in)

| Pin | Wire color | Measured | Signal |
| --- | ---------- | -------- | ------ |
| 1   |            |          |        |
| 2   |            |          |        |
| 3   |            |          |        |

Once confirmed, flip `verified` notes in
`definitions/mv/5sm-brutale-910.toml` accordingly.
