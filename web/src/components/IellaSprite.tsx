export interface IellaSpriteProps {
  size?: number;
  decorative?: boolean;
  className?: string;
}

export function IellaSprite({
  size = 64,
  decorative = true,
  className = "",
}: IellaSpriteProps) {
  return (
    <svg
      className={`iella-sprite ${className}`.trim()}
      width={size}
      height={(size * 14) / 16}
      viewBox="0 0 16 14"
      shapeRendering="crispEdges"
      role={decorative ? undefined : "img"}
      aria-hidden={decorative || undefined}
      aria-label={decorative ? undefined : "Iella"}
    >
      <g className="iella-bob">
        <rect className="iella-ink" x="6" y="0" width="4" height="1" />
        <rect className="iella-ink" x="5" y="1" width="6" height="1" />
        <rect className="iella-beak" x="11" y="1" width="1" height="1" />
        <rect className="iella-ink" x="4" y="2" width="8" height="1" />
        <rect className="iella-ink" x="3" y="3" width="10" height="1" />
        <rect className="iella-ink" x="3" y="4" width="10" height="1" />
        <rect className="iella-cream" x="5" y="3" width="2" height="1" />
        <rect className="iella-eye" x="7" y="3" width="1" height="1" />
        <rect className="iella-cream" x="9" y="4" width="3" height="1" />
        <rect className="iella-ink" x="2" y="5" width="12" height="1" />
        <rect className="iella-cream" x="4" y="5" width="2" height="1" />
        <rect className="iella-beak" x="7" y="5" width="2" height="1" />
        <rect className="iella-cream" x="9" y="5" width="3" height="1" />
        <rect className="iella-ink" x="2" y="6" width="12" height="1" />
        <rect className="iella-cream" x="4" y="6" width="8" height="1" />
        <rect className="iella-ink" x="3" y="7" width="10" height="1" />
        <rect className="iella-cream" x="4" y="7" width="8" height="1" />
        <rect className="iella-ink" x="4" y="8" width="8" height="1" />
        <rect className="iella-cream" x="5" y="8" width="6" height="1" />
        <rect className="iella-ink" x="5" y="9" width="6" height="1" />
        <rect className="iella-brown" x="5" y="10" width="2" height="1" />
        <rect className="iella-brown" x="9" y="10" width="2" height="1" />
        <rect className="iella-beak" x="12" y="6" width="1" height="2" />
        <rect className="iella-cream" x="13" y="7" width="1" height="1" />
      </g>
    </svg>
  );
}
