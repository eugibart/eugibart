import { AccessZone, BodyStyle } from "./ipc";

/**
 * Schematic side-view motorcycle (front to the right) with an animated
 * marker on the requested access zone. Deliberately line-art, not a photo:
 * locations are approximate community knowledge, and a stylized diagram
 * says so visually where a photograph would over-promise.
 *
 * The pulse ring is decorative (CSS animation, killed by the global
 * prefers-reduced-motion rule); the accessible content is the aria-label
 * plus the visible caption HowToPanel renders next to this.
 */

export const ZONE_LABELS: Record<AccessZone, string> = {
  "under-seat": "under the seat",
  "under-tank-left": "under the tank — left side",
  "under-tank-right": "under the tank — right side",
  "under-tank-center": "under the tank — center",
  "tail-section": "in the tail section",
  "dash-area": "near the dash / headstock",
  "side-panel-left": "behind the left side panel",
  "side-panel-right": "behind the right side panel",
};

/** Marker positions per silhouette, in the 400×220 viewBox. */
export const ZONE_COORDS: Record<BodyStyle, Record<AccessZone, { x: number; y: number }>> = {
  naked: {
    "under-seat": { x: 152, y: 104 },
    "under-tank-left": { x: 212, y: 99 },
    "under-tank-right": { x: 212, y: 99 },
    "under-tank-center": { x: 228, y: 96 },
    "tail-section": { x: 104, y: 92 },
    "dash-area": { x: 283, y: 64 },
    "side-panel-left": { x: 188, y: 116 },
    "side-panel-right": { x: 188, y: 116 },
  },
  faired: {
    "under-seat": { x: 152, y: 104 },
    "under-tank-left": { x: 212, y: 99 },
    "under-tank-right": { x: 212, y: 99 },
    "under-tank-center": { x: 228, y: 96 },
    "tail-section": { x: 102, y: 86 },
    "dash-area": { x: 280, y: 60 },
    "side-panel-left": { x: 216, y: 132 },
    "side-panel-right": { x: 216, y: 132 },
  },
};

/* Shared stance: wheels, fork, frame, engine, swingarm. Angular
   blueprint-style segments — deliberate, not sketchy. */
function Rolling() {
  return (
    <>
      <circle cx="92" cy="158" r="38" />
      <circle cx="92" cy="158" r="8" />
      <circle cx="308" cy="158" r="38" />
      <circle cx="308" cy="158" r="8" />
      {/* fork */}
      <path d="M308 158 L281 74" />
      {/* frame tubes */}
      <path d="M262 92 L224 126 M184 106 L192 126" />
      {/* engine */}
      <rect x="156" y="126" width="76" height="34" rx="8" />
      {/* swingarm */}
      <path d="M92 158 L158 152" />
    </>
  );
}

function NakedSilhouette() {
  return (
    <g className="bike-line">
      <Rolling />
      {/* upright handlebar */}
      <path d="M276 68 L298 58" />
      {/* body top: tail tip → seat → tank peak → headstock */}
      <path d="M92 84 L138 94 L180 94 L204 76 L236 68 L266 72" />
      {/* body bottom: headstock → tank underside → seat base → tail */}
      <path d="M266 72 L268 90 L200 104 L134 108 L98 98 L92 84" />
      {/* low exhaust muffler */}
      <path d="M152 163 L112 160" />
    </g>
  );
}

function FairedSilhouette() {
  return (
    <g className="bike-line">
      <Rolling />
      {/* windscreen instead of an upright bar */}
      <path d="M272 72 L290 52" />
      {/* body top: raised tail → seat → tank → headstock */}
      <path d="M96 74 L140 92 L180 92 L204 74 L236 66 L266 70" />
      {/* body bottom */}
      <path d="M266 70 L268 90 L200 104 L134 108 L102 96 L96 74" />
      {/* underseat exhaust tips (F4 organ pipes) */}
      <path d="M104 82 L91 78 M103 90 L90 88" />
      {/* belly pan under the engine */}
      <path d="M240 150 L234 168 L170 170 L150 156" />
    </g>
  );
}

export default function BikeDiagram({
  zone,
  bodyStyle,
}: {
  zone: AccessZone;
  bodyStyle: BodyStyle;
}) {
  const { x, y } = ZONE_COORDS[bodyStyle][zone];
  return (
    <svg
      className="bike-diagram"
      viewBox="0 0 400 220"
      role="img"
      aria-label={`Approximate location: ${ZONE_LABELS[zone]}`}
    >
      {bodyStyle === "faired" ? <FairedSilhouette /> : <NakedSilhouette />}
      <g className="bike-marker">
        <circle className="bike-pulse" cx={x} cy={y} r="10" />
        <circle className="bike-pulse bike-pulse-late" cx={x} cy={y} r="10" />
        <circle className="bike-marker-ring" cx={x} cy={y} r="10" />
        <circle className="bike-marker-dot" cx={x} cy={y} r="4.5" />
      </g>
    </svg>
  );
}
