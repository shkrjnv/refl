export type IellaMood = "idle" | "perched" | "curious" | "proud";
export type IellaLook = "left" | "center" | "right";

interface IellaBaseProps {
  size?: number;
  decorative?: boolean;
  className?: string;
  mood?: IellaMood;
  look?: IellaLook;
  onHover?: () => void;
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

const GRID_WIDTH = 12;
const GRID_HEIGHT = 11;

export function IellaSprite({
  size = 60,
  decorative = true,
  className = "",
  label,
  onActivate,
  onHover,
  mood = "idle",
  look = "center",
}: IellaSpriteProps) {
  const interactive = typeof onActivate === "function";
  const scale = Math.max(1, Math.round(size / GRID_WIDTH));
  const width = scale * GRID_WIDTH;
  const height = scale * GRID_HEIGHT;
  const sprite = (
    <svg
      className={`iella-sprite ${className}`.trim()}
      width={width}
      height={height}
      viewBox="0 0 12 11"
      shapeRendering="crispEdges"
      overflow="hidden"
      data-mood={mood}
      data-look={look}
      data-perched={mood === "perched" ? "true" : "false"}
      role={!interactive && !decorative ? "img" : undefined}
      aria-hidden={interactive || decorative ? true : undefined}
      aria-label={!interactive && !decorative ? label : undefined}
    >
      <g className="iella-breathe">
        <g className="iella-head">
          <rect className="iella-ink" x="6" y="0" width="3" height="1" />
          <rect className="iella-ink" x="5" y="1" width="5" height="1" />
          <rect className="iella-beak" x="10" y="1" width="1" height="1" />
          <rect className="iella-ink" x="5" y="2" width="1" height="1" />
          <g className="iella-eye">
            <rect className="iella-eye-fill" x="6" y="2" width="1" height="1" />
          </g>
          <rect className="iella-ink" x="7" y="2" width="3" height="1" />
          <rect className="iella-beak" x="10" y="2" width="2" height="1" />
          <rect className="iella-ink" x="5" y="3" width="5" height="1" />
          <rect className="iella-cream" x="6" y="4" width="3" height="1" />
          <rect className="iella-ink" x="9" y="4" width="1" height="1" />
        </g>
        <g className="iella-wing iella-wing-left">
          <rect className="iella-ink" x="4" y="4" width="2" height="1" />
          <rect className="iella-ink" x="0" y="4" width="2" height="1" />
          <rect className="iella-ink" x="0" y="5" width="5" height="1" />
          <rect className="iella-cream" x="5" y="5" width="1" height="1" />
          <rect className="iella-ink" x="1" y="6" width="4" height="1" />
          <rect className="iella-ink" x="2" y="7" width="4" height="1" />
          <rect className="iella-ink" x="3" y="8" width="4" height="1" />
        </g>
        <g className="iella-body">
          <rect className="iella-beak" x="6" y="5" width="2" height="1" />
          <rect className="iella-cream" x="8" y="5" width="2" height="1" />
          <rect className="iella-cream" x="5" y="6" width="5" height="1" />
          <rect className="iella-cream" x="6" y="7" width="3" height="1" />
          <rect className="iella-brown" x="9" y="7" width="1" height="1" />
          <rect className="iella-cream" x="7" y="8" width="1" height="1" />
          <rect className="iella-brown" x="8" y="8" width="3" height="1" />
        </g>
        <g className="iella-tail">
          <rect className="iella-ink" x="5" y="9" width="2" height="1" />
          <rect className="iella-brown" x="8" y="9" width="3" height="1" />
        </g>
        <g className="iella-feet">
          <rect className="iella-ink" x="5" y="10" width="1" height="1" />
          <rect className="iella-brown" x="9" y="10" width="1" height="1" />
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
        onPointerEnter={onHover}
      >
        {sprite}
      </button>
    );
  }
  return sprite;
}
