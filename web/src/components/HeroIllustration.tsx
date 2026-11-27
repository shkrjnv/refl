export function HeroIllustration() {
  return (
    <svg
      className="hero-illustration"
      viewBox="0 0 560 300"
      role="presentation"
      aria-hidden="true"
      preserveAspectRatio="xMidYMid slice"
    >
      <defs>
        <linearGradient id="hero-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" className="hero-sky-top" />
          <stop offset="0.62" className="hero-sky-mid" />
          <stop offset="1" className="hero-sky-bottom" />
        </linearGradient>
      </defs>
      <rect width="560" height="300" fill="url(#hero-sky)" />
      <circle className="hero-sun" cx="448" cy="212" r="52" />
      <circle className="hero-sun-glow" cx="448" cy="212" r="72" />
      <path
        className="hero-hill-back"
        d="M0 238 C90 204 164 228 244 208 C324 188 396 230 476 204 C512 192 536 198 560 194 L560 300 L0 300Z"
      />
      <path
        className="hero-hill-front"
        d="M0 262 C108 238 196 258 300 242 C404 226 472 250 560 236 L560 300 L0 300Z"
      />
      <path
        className="hero-route"
        d="M74 246 C142 246 142 188 210 188 C278 188 278 226 346 226 C408 226 408 166 474 166"
      />
      <g className="hero-route-dot hero-route-dot-a">
        <rect x="62" y="234" width="24" height="24" rx="8" />
        <path d="M68 246l5 5 9-11" />
      </g>
      <g className="hero-route-dot hero-route-dot-b">
        <rect x="198" y="176" width="24" height="24" rx="8" />
        <circle cx="210" cy="188" r="4" />
      </g>
      <g className="hero-route-dot hero-route-dot-c">
        <rect x="334" y="214" width="24" height="24" rx="8" />
        <path d="M341 226h10M346 221v10" />
      </g>
      <g className="hero-cloud hero-cloud-a">
        <circle cx="84" cy="78" r="18" />
        <circle cx="108" cy="68" r="24" />
        <circle cx="134" cy="80" r="16" />
        <rect x="70" y="76" width="80" height="18" rx="9" />
      </g>
      <g className="hero-cloud hero-cloud-b">
        <circle cx="356" cy="54" r="14" />
        <circle cx="374" cy="46" r="19" />
        <circle cx="394" cy="56" r="12" />
        <rect x="346" y="54" width="60" height="14" rx="7" />
      </g>
      <g className="hero-star hero-star-a">
        <path d="M270 42c1 7 3 9 10 10-7 1-9 3-10 10-1-7-3-9-10-10 7-1 9-3 10-10Z" />
      </g>
      <g className="hero-star hero-star-b">
        <path d="M494 74c1 5 2 6 7 7-5 1-6 2-7 7-1-5-2-6-7-7 5-1 6-2 7-7Z" />
      </g>
    </svg>
  );
}
