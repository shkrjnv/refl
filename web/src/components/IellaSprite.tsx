export type IellaMood = "idle" | "flying" | "perched" | "curious" | "proud";
export type IellaLook = "left" | "center" | "right";

interface IellaBaseProps {
  size?: number;
  decorative?: boolean;
  className?: string;
  mood?: IellaMood;
  look?: IellaLook;
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
  mood = "idle",
  look = "center",
}: IellaSpriteProps) {
  const interactive = typeof onActivate === "function";
  const sprite = (
    <svg
      className={`iella-sprite ${className}`.trim()}
      width={size}
      height={(size * 19) / 24}
      viewBox="-2 -2 24 19"
      shapeRendering="crispEdges"
      overflow="visible"
      data-mood={mood}
      data-look={look}
      data-perched={mood === "perched" ? "true" : "false"}
      role={!interactive && !decorative ? "img" : undefined}
      aria-hidden={interactive || decorative ? true : undefined}
      aria-label={!interactive && !decorative ? label : undefined}
    >
      <g className="iella-bob">
        <g className="iella-feet">
          <rect className="iella-brown" x="6" y="14" width="2" height="1" />
          <rect className="iella-brown" x="14" y="14" width="2" height="1" />
          <rect className="iella-ink" x="5" y="15" width="3" height="1" />
          <rect className="iella-ink" x="14" y="15" width="3" height="1" />
        </g>
        <g className="iella-tail">
          <rect className="iella-ink" x="16" y="10" width="3" height="1" />
          <rect className="iella-brown" x="17" y="11" width="3" height="1" />
          <rect className="iella-ink" x="18" y="12" width="2" height="1" />
        </g>
        <g className="iella-body">
          <rect className="iella-ink" x="3" y="5" width="16" height="6" />
          <rect className="iella-ink" x="4" y="11" width="14" height="1" />
          <rect className="iella-ink" x="6" y="12" width="10" height="1" />
          <rect className="iella-cream" x="4" y="5" width="14" height="6" />
          <rect className="iella-brown" x="7" y="10" width="8" height="1" />
          <rect className="iella-cream" x="8" y="11" width="6" height="1" />
          <rect className="iella-ink" x="8" y="13" width="6" height="1" />
        </g>
        <g className="iella-wing iella-wing-left">
          <rect className="iella-ink" x="1" y="7" width="4" height="1" />
          <rect className="iella-ink" x="0" y="8" width="4" height="2" />
          <rect className="iella-cream" x="1" y="8" width="2" height="1" />
          <rect className="iella-beak" x="1" y="10" width="2" height="1" />
        </g>
        <g className="iella-wing iella-wing-right">
          <rect className="iella-ink" x="17" y="7" width="4" height="1" />
          <rect className="iella-ink" x="18" y="8" width="4" height="2" />
          <rect className="iella-cream" x="19" y="8" width="2" height="1" />
          <rect className="iella-beak" x="19" y="10" width="2" height="1" />
        </g>
        <g className="iella-head">
          <rect className="iella-ink" x="7" y="2" width="8" height="1" />
          <rect className="iella-ink" x="5" y="3" width="12" height="1" />
          <rect className="iella-ink" x="4" y="4" width="14" height="1" />
          <rect className="iella-cream" x="7" y="4" width="8" height="1" />
          <g className="iella-eyes">
            <g className="iella-eye">
              <rect className="iella-eye-fill" x="7" y="5" width="2" height="2" />
              <rect className="iella-cream" x="8" y="5" width="1" height="1" />
            </g>
            <g className="iella-eye">
              <rect className="iella-eye-fill" x="13" y="5" width="2" height="2" />
              <rect className="iella-cream" x="14" y="5" width="1" height="1" />
            </g>
          </g>
          <g className="iella-happy-eyes" aria-hidden="true">
            <rect className="iella-ink" x="7" y="6" width="2" height="1" />
            <rect className="iella-ink" x="13" y="6" width="2" height="1" />
          </g>
          <rect className="iella-beak" x="9" y="7" width="4" height="1" />
          <rect className="iella-brown" x="10" y="8" width="2" height="1" />
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
