import { AccessZone, BodyStyle } from "./ipc";

/**
 * Stylized side-view line-art of the two body styles the app covers — a
 * Brutale-shaped naked (round headlight, muscular tank, trellis frame,
 * stacked right-side silencers, single-sided swingarm) and an F4-shaped
 * sportbike (nose fairing, screen, underseat organ pipes) — with an
 * animated marker on the requested access zone. Deliberately an
 * illustration, not a photo: locations are approximate community
 * knowledge, and the drawing's style says so while still letting an owner
 * recognize their bike.
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
    "under-seat": { x: 150, y: 96 },
    "under-tank-left": { x: 218, y: 102 },
    "under-tank-right": { x: 218, y: 102 },
    "under-tank-center": { x: 233, y: 100 },
    "tail-section": { x: 118, y: 88 },
    "dash-area": { x: 281, y: 52 },
    "side-panel-left": { x: 186, y: 112 },
    "side-panel-right": { x: 186, y: 112 },
  },
  faired: {
    "under-seat": { x: 162, y: 92 },
    "under-tank-left": { x: 214, y: 96 },
    "under-tank-right": { x: 214, y: 96 },
    "under-tank-center": { x: 228, y: 94 },
    "tail-section": { x: 90, y: 60 },
    "dash-area": { x: 272, y: 54 },
    "side-panel-left": { x: 262, y: 130 },
    "side-panel-right": { x: 262, y: 130 },
  },
};

/* The shared drawing frame both silhouettes are plotted in. The real bikes'
   ~300 mm wheel radius mapped onto WHEEL_R fixes the scale at 7.5 mm per
   viewBox unit; the tires touch at GROUND_Y, and the baseline is drawn two
   units lower so it doesn't overlap the tire strokes. */
const WHEEL_R = 40;
const WHEEL_CY = 156;
const FRONT_CX = 312;
const REAR_CX = 88;
const GROUND_Y = WHEEL_CY + WHEEL_R;
const GROUND_LINE = `M44 ${GROUND_Y + 2} L356 ${GROUND_Y + 2}`;

/** Spoked wheel: tire, rim, five spokes, hub. */
function Wheel({ cx }: { cx: number }) {
  const s = (dx1: number, dy1: number, dx2: number, dy2: number) =>
    `M${cx + dx1} ${WHEEL_CY + dy1} L${cx + dx2} ${WHEEL_CY + dy2}`;
  return (
    <>
      <circle className="bike-line" cx={cx} cy={WHEEL_CY} r={WHEEL_R} />
      <g className="bike-thin">
        <circle cx={cx} cy={WHEEL_CY} r="26" />
        <path
          d={[s(0, -8, 0, -25), s(-8, 3, -24, 8), s(5, 7, 15, 21), s(8, -6, 22, -15), s(-6, -6, -18, -18)].join(
            " ",
          )}
        />
      </g>
      <circle className="bike-line" cx={cx} cy={WHEEL_CY} r="7" />
    </>
  );
}

function NakedSilhouette() {
  return (
    <g>
      <Wheel cx={FRONT_CX} />
      <Wheel cx={REAR_CX} />
      {/* fork + triple clamp + front fender */}
      <g className="bike-line">
        <path d="M313 152 L291 68" />
        <path d="M306 150 L284 66" />
        <path d="M284 66 L292 68" />
        <path d="M287 125 Q312 98 337 125" />
      </g>
      {/* round headlight at the crown */}
      <ellipse className="bike-line" cx="293" cy="60" rx="7" ry="9" />
      {/* riser + wide bar */}
      <g className="bike-line">
        <path d="M285 62 L280 50" />
        <path d="M286 48 L252 42" />
      </g>
      {/* muscular tank */}
      <path
        className="bike-fill"
        d="M276 70 C260 55 242 53 230 58 C214 64 202 76 195 88 C205 97 224 101 242 99 C258 97 269 89 274 80 Z"
      />
      {/* seat + upswept tail */}
      <path
        className="bike-fill"
        d="M198 87 C180 91 162 92 148 91 C136 90 126 86 118 78 L124 94 C134 101 148 102 162 100 L193 97 Z"
      />
      {/* trellis frame */}
      <g className="bike-thin">
        <path d="M262 96 L206 118" />
        <path d="M256 106 L204 128" />
        <path d="M246 100 L242 115" />
        <path d="M230 106 L226 122" />
        <path d="M216 112 L212 125" />
      </g>
      {/* engine + clutch cover */}
      <path
        className="bike-line"
        d="M200 118 L243 112 L250 130 L246 156 C238 164 200 166 180 162 L164 152 L166 130 Z"
      />
      <g className="bike-thin">
        <circle cx="196" cy="142" r="12" />
        <circle cx="196" cy="142" r="4" />
        <path d="M243 112 L250 98 M230 115 L237 101" />
      </g>
      {/* exhaust headers sweeping under the engine */}
      <g className="bike-line">
        <path d="M246 102 C258 112 260 130 252 144 C246 154 236 160 226 163 C212 167 196 167 184 163" />
        <path d="M184 163 C178 158 173 151 170 144" />
      </g>
      {/* single-sided swingarm (thin — the cans dominate this side) */}
      <g className="bike-thin">
        <path d="M184 130 L96 150 M188 142 L98 158" />
      </g>
      {/* twin stacked right-side silencers */}
      <path
        className="bike-fill"
        d="M166 122 L126 127 A5.5 5.5 0 0 0 127 138 L167 133 A5.5 5.5 0 0 0 166 122 Z"
      />
      <path
        className="bike-fill"
        d="M170 136 L130 141 A5.5 5.5 0 0 0 131 152 L171 147 A5.5 5.5 0 0 0 170 136 Z"
      />
      <path className="bike-thin" d={GROUND_LINE} />
    </g>
  );
}

/* Dimension-true to the factory figures (via a schematic reference), in the
   shared frame's 7.5 mm/unit scale: screen top at the 1165 mm line, seat at
   810 mm, belly at 120 mm ground clearance, beak nose and tail overhangs
   from the 2060 mm overall length — all measured up from GROUND_Y. */
function FairedSilhouette() {
  return (
    <g>
      <Wheel cx={FRONT_CX} />
      <Wheel cx={REAR_CX} />
      {/* raked fork in the pocket between fender and beak + front fender */}
      <g className="bike-line">
        <path d="M313 152 L302 124" />
        <path d="M306 150 L295 122" />
        <path d="M287 124 Q312 102 337 124" />
      </g>
      {/* upper body: tank -> saddle -> long tail past the rear axle */}
      <path
        className="bike-fill"
        d="M266 62 C250 66 218 66 202 64 C192 65 186 70 182 74 C174 84 166 87 158 87 C144 78 130 64 116 54 L62 50 C54 51 52 55 55 60 L58 68 C74 70 92 72 110 74 C126 77 138 82 148 88 C160 93 170 95 178 96 C200 98 222 96 240 90 C252 85 260 74 266 62 Z"
      />
      {/* fairing: beak nose, lower edge wrapping behind the front wheel,
          flat 120 mm belly, vertical rear cut */}
      <path
        className="bike-fill"
        d="M266 62 L316 58 C332 64 342 74 344 84 C345 90 341 95 334 97 C324 106 312 112 302 120 C290 138 278 154 262 166 C242 176 216 180 200 178 L196 108 C210 100 228 95 240 90 C250 85 260 74 266 62 Z"
      />
      {/* windscreen up to the 1165 mm overall-height line */}
      <path className="bike-fill" d="M287 42 C296 46 306 52 314 58 L268 62 C273 53 280 46 287 42 Z" />
      {/* headlight slit + fairing seam */}
      <g className="bike-thin">
        <path d="M330 68 C336 74 340 80 341 86" />
        <path d="M298 84 C280 106 256 132 238 158" />
      </g>
      {/* organ pipes at the tail rear */}
      <path className="bike-line" d="M60 62 L48 61 M60 68 L48 67" />
      <g className="bike-thin">
        <circle cx="47" cy="61" r="2.4" />
        <circle cx="47" cy="67" r="2.4" />
      </g>
      {/* rear hugger */}
      <path className="bike-thin" d="M64 128 Q88 112 112 126" />
      {/* engine cases in the cutout behind the fairing */}
      <rect className="bike-line" x="156" y="104" width="42" height="38" rx="7" />
      <g className="bike-thin">
        <circle cx="176" cy="126" r="8" />
        <path d="M160 114 L196 112" />
      </g>
      {/* single-sided swingarm */}
      <path className="bike-fill" d="M158 136 L96 148 L98 160 L162 148 Z" />
      <path className="bike-thin" d={GROUND_LINE} />
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
