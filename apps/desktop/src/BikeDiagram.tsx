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
    "under-seat": { x: 162, y: 82 },
    "under-tank-left": { x: 215, y: 92 },
    "under-tank-right": { x: 215, y: 92 },
    "under-tank-center": { x: 228, y: 90 },
    "tail-section": { x: 90, y: 55 },
    "dash-area": { x: 268, y: 46 },
    "side-panel-left": { x: 266, y: 122 },
    "side-panel-right": { x: 266, y: 122 },
  },
};

/** Spoked wheel: tire, rim, five spokes, hub. */
function Wheel({ cx }: { cx: number }) {
  const s = (dx1: number, dy1: number, dx2: number, dy2: number) =>
    `M${cx + dx1} ${156 + dy1} L${cx + dx2} ${156 + dy2}`;
  return (
    <>
      <circle className="bike-line" cx={cx} cy="156" r="40" />
      <g className="bike-thin">
        <circle cx={cx} cy="156" r="26" />
        <path
          d={[s(0, -8, 0, -25), s(-8, 3, -24, 8), s(5, 7, 15, 21), s(8, -6, 22, -15), s(-6, -6, -18, -18)].join(
            " ",
          )}
        />
      </g>
      <circle className="bike-line" cx={cx} cy="156" r="7" />
    </>
  );
}

function NakedSilhouette() {
  return (
    <g>
      <Wheel cx={312} />
      <Wheel cx={88} />
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
      <path className="bike-thin" d="M44 198 L356 198" />
    </g>
  );
}

/* Proportions measured from a reference photo of an F4 1000 (wheelbase-
   normalized): level tank flowing into the long, high tail that overhangs
   the rear wheel, tall far-forward screen, deep lower fairing with the
   red/silver seam, organ-pipe tips at the very rear of the tail. */
function FairedSilhouette() {
  return (
    <g>
      <Wheel cx={312} />
      <Wheel cx={88} />
      {/* fork visible below the nose + front fender */}
      <g className="bike-line">
        <path d="M313 152 L305 118" />
        <path d="M306 150 L298 116" />
        <path d="M287 125 Q312 98 337 125" />
      </g>
      {/* upper body: tank -> saddle -> tail overhanging the rear wheel */}
      <path
        className="bike-fill"
        d="M272 52 C252 59 216 56 198 55 C190 57 186 60 182 63 C174 71 166 73 158 73 C142 66 126 54 110 46 L64 43 C56 44 54 48 57 53 L61 63 C76 64 94 66 112 68 C128 71 140 76 150 80 C162 85 172 87 180 88 C202 91 224 89 242 84 C254 79 264 64 272 52 Z"
      />
      {/* fairing: nose with headlight face, low belly, vertical rear cut */}
      <path
        className="bike-fill"
        d="M272 52 L316 50 C326 56 332 74 330 90 C328 98 322 102 316 102 C300 122 278 142 256 154 C236 163 214 166 198 163 L196 104 C210 96 232 90 244 84 C254 78 264 62 272 52 Z"
      />
      {/* windscreen: tall, far forward */}
      <path className="bike-fill" d="M290 28 C300 36 310 43 318 49 L272 53 C277 43 283 34 290 28 Z" />
      {/* headlight slit + fairing/tank seam */}
      <g className="bike-thin">
        <path d="M320 58 C326 68 329 80 328 90" />
        <path d="M300 76 C282 96 258 122 240 146" />
      </g>
      {/* organ pipes exiting the tail rear */}
      <path className="bike-line" d="M62 57 L50 56 M62 63 L50 62" />
      <g className="bike-thin">
        <circle cx="49" cy="56" r="2.4" />
        <circle cx="49" cy="62" r="2.4" />
      </g>
      {/* rear hugger */}
      <path className="bike-thin" d="M66 126 Q88 110 110 124" />
      {/* engine cases in the cutout behind the fairing */}
      <rect className="bike-line" x="158" y="100" width="40" height="36" rx="7" />
      <g className="bike-thin">
        <circle cx="176" cy="120" r="8" />
        <path d="M162 108 L196 106" />
      </g>
      {/* single-sided swingarm */}
      <path className="bike-fill" d="M158 134 L96 148 L98 160 L162 146 Z" />
      <path className="bike-thin" d="M44 198 L356 198" />
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
