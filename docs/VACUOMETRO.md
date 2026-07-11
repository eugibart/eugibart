# Vacuometro — digital vacuum gauge for throttle-body sync

## Why a vacuum gauge at all

Throttle-body synchronization (*sincronizzazione dei corpi farfallati*) means
adjusting the balance screws until every cylinder pulls the same intake
vacuum at idle. It is the mandatory precursor to the CO adjustment: with
unbalanced bodies, cylinders do unequal work, idle hunts, and any CO trim you
set is chasing a moving target. Sync first, CO after — this is exactly the
procedure Italian owner forums describe for the MV F4/Brutale family
(daidegasforum, motoclub-tingavert) and it matches workshop-manual practice.

Traditionally you watch a bank of analog gauges (or mercury sticks) with one
eye and the tachometer with the other. MotoDiag's **Sync screen** replaces
that juggling act: per-cylinder vacuum bars, the spread number to drive
toward zero, and live RPM from the ECU session — one screen, and everything
flows into the CSV log with a shared timeline.

## What MotoDiag connects to

Analog gauges (Carbtune, TwinMax, dial sets) have no data output. MotoDiag
instead reads an **open-hardware digital gauge** over a second USB port —
independent of the K-line cable, so both are plugged in at once:

```
bike ──[K-line KKL cable]──────────► Mac ◄──────────[USB]── vacuum gauge box
      vacuum ports ──hoses──► MAP sensors ──► ADC ──► microcontroller ┘
```

## Build it (~€50)

| Part | Qty | Notes |
| ---- | --- | ----- |
| Arduino Uno/Nano (or clone) | 1 | Any 5 V board; ESP32 works with care (see firmware header) |
| ADS1115 16-bit I2C ADC breakout | 1 | 4 channels, one per cylinder |
| NXP **MPX4250AP** absolute pressure sensor | 4 | 20–250 kPa; the de-facto standard in DIY sync tools |
| Silicone vacuum hose 4 mm + tees | — | To the throttle-body vacuum ports |
| Restrictors (0.6 mm orifice or fuel-filter foam) | 4 | Damps combustion pulsation — do not skip |
| USB cable | 1 | To the Mac |

Wiring: each MPX4250AP → 5 V/GND plus its Vout to ADS1115 A0–A3; ADS1115 →
I2C (A4/A5 on an Uno/Nano). Flash `tools/vacuum-gauge-firmware/vacuum_gauge.ino`
(needs the Adafruit ADS1X15 library). The board then streams:

```
HELLO,motodiag-vac,4
VAC,31.2,31.5,30.9,31.4      ← absolute kPa per cylinder, ~10 Hz
```

Anything else on the line is ignored by MotoDiag's parser, so debug prints
are harmless. If you already own one of the community Arduino sync tools
(Instructables shield, SyncCarb, digitalcarbsync), porting it is one line:
print `VAC,` + comma-separated kPa.

## Using the Sync screen

1. Connect the K-line cable and the gauge box (two USB ports).
2. Connect tab → connect to the bike as usual.
3. Sync tab → pick the gauge's port → **Connect gauge**. (No hardware? Pick
   "Simulated vacuum gauge" to explore the screen.)
4. Follow the on-screen procedure: warm engine, balance pair 1–2, pair 3–4,
   then the center screw, driving the **spread** into the green (< 0.5 kPa,
   an indicative threshold — your workshop manual's word wins).
5. With sync done, adjust idle CO (Service tab, once the CO routine is
   verified on real hardware — see docs/PROTOCOL.md discovery plan).

Readings are absolute manifold pressure: **lower kPa = stronger vacuum**. An
idling engine typically sits around 25–40 kPa; ~100 kPa means that hose is
open to atmosphere (not connected).

## Safety notes

- The gauge is read-only and electrically separate from the ECU — worst case
  a bad sensor shows nonsense, it cannot affect the bike.
- Sync with the engine warm and a fan on the radiator; prolonged idling
  overheats these engines quickly.
- Cap or reconnect the vacuum ports when done.
