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
    "under-seat": { x: 152, y: 98 },
    "under-tank-left": { x: 218, y: 100 },
    "under-tank-right": { x: 218, y: 100 },
    "under-tank-center": { x: 230, y: 96 },
    "tail-section": { x: 124, y: 86 },
    "dash-area": { x: 258, y: 54 },
    "side-panel-left": { x: 272, y: 112 },
    "side-panel-right": { x: 272, y: 112 },
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

function FairedSilhouette() {
  return (
    <g>
      <Wheel cx={312} />
      <Wheel cx={88} />
      {/* fork visible below the fairing + fender */}
      <g className="bike-line">
        <path d="M313 152 L303 114" />
        <path d="M306 150 L296 112" />
        <path d="M287 125 Q312 98 337 125" />
      </g>
      {/* body: nose fairing over the fender, belly pan, rear cut, tank top */}
      <path
        className="bike-fill"
        d="M268 56 C288 60 306 74 318 90 C322 96 320 102 314 104 C306 122 292 138 274 146 C258 152 242 152 232 148 L236 116 C232 106 220 100 197 96 C222 88 238 76 250 63 L268 56 Z"
      />
      {/* windscreen */}
      <path className="bike-fill" d="M267 57 L252 41 C256 51 260 58 266 62 Z" />
      {/* headlight slit, intake, fairing/tank seam */}
      <g className="bike-thin">
        <path d="M303 78 C310 85 315 92 317 97" />
        <path d="M276 70 L292 80" />
        <path d="M262 60 C252 78 244 98 239 122" />
      </g>
      {/* seat + tall tail with the four underseat organ pipes */}
      <path
        className="bike-fill"
        d="M200 95 C182 97 164 96 152 94 C140 92 128 86 120 76 L128 95 C137 102 152 104 164 102 L195 101 Z"
      />
      <g className="bike-thin">
        <circle cx="131" cy="84" r="3.2" />
        <circle cx="138" cy="87" r="3.2" />
        <circle cx="129" cy="91" r="3.2" />
        <circle cx="136" cy="94" r="3.2" />
      </g>
      {/* clip-on */}
      <path className="bike-thin" d="M272 62 L283 58" />
      {/* engine visible in the rear cutout */}
      <g className="bike-line">
        <rect x="202" y="116" width="32" height="30" rx="6" />
        <path d="M232 148 C220 154 204 155 192 150 L182 142" />
      </g>
      <circle className="bike-thin" cx="212" cy="136" r="7" />
      {/* frame glimpse between fairing and seat */}
      <path className="bike-thin" d="M222 96 L204 116 M212 100 L220 114" />
      {/* single-sided swingarm */}
      <path className="bike-fill" d="M182 128 L96 147 L98 159 L184 141 Z" />
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
