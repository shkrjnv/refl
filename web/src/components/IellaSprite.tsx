interface IellaBaseProps {
  size?: number;
  decorative?: boolean;
  className?: string;
}

interface IellaStaticProps extends IellaBaseProps {
  onActivate?: undefined;
  label?: string;
}

interface IellaButtonProps extends IellaBaseProps {
  onActivate: () => void;
  label: string;
  decorative?: false;
}

export type IellaSpriteProps = IellaStaticProps | IellaButtonProps;

export function IellaSprite({
  size = 64,
  decorative = true,
  className = "",
  label,
  onActivate,
}: IellaSpriteProps) {
  const interactive = typeof onActivate === "function";
  const sprite = (
    <svg
      className={`iella-sprite ${className}`.trim()}
      width={size}
      height={(size * 15) / 16}
      viewBox="-1 -2 16 15"
      shapeRendering="crispEdges"
      overflow="visible"
      role={!interactive && !decorative ? "img" : undefined}
      aria-hidden={interactive || decorative ? true : undefined}
      aria-label={!interactive && !decorative ? label : undefined}
    >
      <g className="iella-bob">
        <g className="iella-body">
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
        </g>
        <g className="iella-head">
          <rect className="iella-ink" x="6" y="0" width="4" height="1" />
          <rect className="iella-ink" x="5" y="1" width="6" height="1" />
          <rect className="iella-beak" x="11" y="1" width="1" height="1" />
          <rect className="iella-ink" x="4" y="2" width="8" height="1" />
          <rect className="iella-ink" x="3" y="3" width="10" height="1" />
          <rect className="iella-ink" x="3" y="4" width="10" height="1" />
          <rect className="iella-cream" x="5" y="3" width="2" height="1" />
          <rect className="iella-cream" x="9" y="4" width="3" height="1" />
          <rect className="iella-eye" x="7" y="3" width="1" height="1" />
        </g>
        <g className="iella-tail">
          <rect className="iella-beak" x="12" y="6" width="1" height="2" />
          <rect className="iella-cream" x="13" y="7" width="1" height="1" />
        </g>
      </g>
    </svg>
  );
  if (onActivate) {
    return (
      <button
        type="button"
        className={`iella-sprite-button ${className}`.trim()}
        aria-label={label}
        onClick={onActivate}
      >
        {sprite}
      </button>
    );
  }
  return sprite;
}
