const STEPS = [
  { x: 92, y: 108, number: "01" },
  { x: 160, y: 218, number: "02" },
  { x: 268, y: 238, number: "03" },
  { x: 374, y: 190, number: "04" },
  { x: 470, y: 168, number: "05" },
] as const;

export function HeroIllustration() {
  return (
    <svg
      className="hero-illustration"
      viewBox="0 0 560 320"
      role="presentation"
      aria-hidden="true"
      preserveAspectRatio="xMidYMid slice"
    >
      <defs>
        <linearGradient id="hero-atmosphere" x1="0" y1="0" x2="0.8" y2="1">
          <stop offset="0" className="hero-atmos-top" />
          <stop offset="0.56" className="hero-atmos-mid" />
          <stop offset="1" className="hero-atmos-bottom" />
        </linearGradient>
        <radialGradient id="hero-glow">
          <stop offset="0" className="hero-glow-core" />
          <stop offset="1" className="hero-glow-edge" />
        </radialGradient>
      </defs>

      <rect width="560" height="320" fill="url(#hero-atmosphere)" />
      <ellipse
        className="hero-celestial-glow"
        cx="470"
        cy="72"
        rx="180"
        ry="132"
        fill="url(#hero-glow)"
      />

      <g className="hero-contours">
        <path
          className="hero-contour hero-contour-a"
          d="M-24 132 C72 100 132 148 222 124 C320 98 376 136 470 108 C510 96 544 102 584 88"
        />
        <path
          className="hero-contour hero-contour-b"
          d="M-18 168 C74 142 138 184 228 160 C326 134 386 174 472 146 C514 132 550 140 586 126"
        />
        <path
          className="hero-contour hero-contour-c"
          d="M-22 206 C70 182 142 218 238 198 C334 178 390 212 480 184 C522 172 554 178 584 166"
        />
        <path
          className="hero-contour hero-contour-d"
          d="M-16 250 C80 228 148 258 242 240 C338 222 400 252 486 226 C526 214 558 220 588 208"
        />
      </g>

      <path
        className="hero-layer hero-layer-far"
        d="M0 238 C72 206 132 229 206 208 C284 186 350 226 426 202 C486 184 524 196 560 188 L560 320 L0 320Z"
      />
      <path
        className="hero-layer hero-layer-mid"
        d="M0 268 C86 240 158 263 246 242 C336 220 408 254 488 232 C522 222 544 226 560 220 L560 320 L0 320Z"
      />
      <path
        className="hero-layer hero-layer-near"
        d="M0 296 C106 272 194 292 296 274 C398 256 470 282 560 262 L560 320 L0 320Z"
      />

      <g className="hero-ribbons">
        <path
          className="hero-ribbon hero-ribbon-a"
          d="M-20 108 C116 36 204 126 322 74 C420 30 502 52 584 116"
        />
        <path
          className="hero-ribbon hero-ribbon-b"
          d="M-20 180 C96 122 202 188 318 148 C410 116 510 140 584 188"
        />
      </g>

      <path
        className="hero-route-line"
        d="M92 108 C128 108 118 218 160 218 C208 218 226 238 268 238 C318 238 328 190 374 190 C414 190 438 168 470 168"
      />

      {STEPS.map((step, index) => (
        <g
          key={step.number}
          className={`hero-step hero-step-${index + 1}`}
          transform={`translate(${step.x} ${step.y})`}
        >
          <circle className="hero-step-halo" r="25" />
          <circle className="hero-step-disc" r="18" />
          <text className="hero-step-number" x="0" y="4">
            {step.number}
          </text>
        </g>
      ))}

      <g className="hero-cloud hero-cloud-a">
        <rect x="58" y="46" width="98" height="18" rx="9" />
        <rect x="80" y="30" width="58" height="16" rx="8" />
      </g>
      <g className="hero-cloud hero-cloud-b">
        <rect x="354" y="46" width="74" height="14" rx="7" />
        <rect x="374" y="34" width="42" height="12" rx="6" />
      </g>
      <g className="hero-star hero-star-a">
        <path d="M276 42c1 6 2 8 8 9-6 1-7 3-8 9-1-6-2-8-8-9 6-1 7-3 8-9Z" />
      </g>
      <g className="hero-star hero-star-b">
        <path d="M500 122c1 4 2 5 6 6-4 1-5 2-6 6-1-4-2-5-6-6 4-1 5-2 6-6Z" />
      </g>
    </svg>
  );
}
