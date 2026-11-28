export type SendoMood =
  | "idle"
  | "happy"
  | "celebrate"
  | "thinking"
  | "sleep"
  | "surprised";

interface SendoBaseProps {
  size?: number;
  decorative?: boolean;
  className?: string;
  mood?: SendoMood;
  onHover?: () => void;
}

interface SendoStaticProps extends SendoBaseProps {
  onActivate?: undefined;
  label?: string;
}

interface SendoButtonProps extends SendoBaseProps {
  onActivate: () => void;
  label: string;
  decorative?: false;
}

export type SendoSpriteProps = SendoStaticProps | SendoButtonProps;

const GRID = 16;

/**
 * Sendo, the moon jerboa keeper of the Infinite Library: silver-blue fur,
 * long ears, round glasses and a firefly lantern. The drawing is a 16×16
 * pixel grid scaled by whole numbers, like Iella.
 */
export function SendoSprite({
  size = 60,
  decorative = true,
  className = "",
  label,
  onActivate,
  onHover,
  mood = "idle",
}: SendoSpriteProps) {
  const interactive = typeof onActivate === "function";
  const scale = Math.max(1, Math.round(size / GRID));
  const width = scale * GRID;
  const height = scale * GRID;
  const sprite = (
    <svg
      className={`sendo-sprite ${className}`.trim()}
      width={width}
      height={height}
      viewBox="0 0 16 16"
      shapeRendering="crispEdges"
      overflow="hidden"
      data-mood={mood}
      role={!interactive && !decorative ? "img" : undefined}
      aria-hidden={interactive || decorative ? true : undefined}
      aria-label={!interactive && !decorative ? label : undefined}
    >
      <g className="sendo-breathe">
        <g className="sendo-ears">
          <rect className="sendo-fur" x="4" y="0" width="2" height="1" />
          <rect className="sendo-fur" x="10" y="0" width="2" height="1" />
          <rect className="sendo-fur" x="3" y="1" width="3" height="1" />
          <rect className="sendo-fur" x="10" y="1" width="3" height="1" />
          <rect className="sendo-fur" x="3" y="2" width="2" height="2" />
          <rect className="sendo-fur" x="11" y="2" width="2" height="2" />
          <rect className="sendo-inner-ear" x="5" y="1" width="1" height="2" />
          <rect className="sendo-inner-ear" x="10" y="1" width="1" height="2" />
        </g>
        <g className="sendo-head">
          <rect className="sendo-fur" x="5" y="4" width="6" height="1" />
          <rect className="sendo-fur" x="4" y="5" width="8" height="1" />
          <rect className="sendo-fur" x="4" y="6" width="8" height="1" />
          <rect className="sendo-fur" x="5" y="7" width="6" height="1" />
          <g className="sendo-glasses">
            <rect className="sendo-glass" x="5" y="5" width="2" height="2" />
            <rect className="sendo-glass" x="9" y="5" width="2" height="2" />
            <rect className="sendo-frame" x="7" y="6" width="2" height="1" />
          </g>
          <g className="sendo-eyes">
            <rect className="sendo-eye" x="6" y="6" width="1" height="1" />
            <rect className="sendo-eye" x="10" y="6" width="1" height="1" />
          </g>
          <rect className="sendo-nose" x="8" y="7" width="1" height="1" />
        </g>
        <g className="sendo-lantern">
          <rect className="sendo-thread" x="13" y="3" width="1" height="3" />
          <rect className="sendo-lantern-glow" x="12" y="6" width="3" height="2" />
          <rect className="sendo-lantern-core" x="13" y="6" width="1" height="1" />
        </g>
        <g className="sendo-body">
          <rect className="sendo-fur" x="5" y="8" width="6" height="1" />
          <rect className="sendo-fur" x="4" y="9" width="8" height="1" />
          <rect className="sendo-fur" x="5" y="10" width="6" height="1" />
          <rect className="sendo-belly" x="6" y="9" width="4" height="2" />
          <rect className="sendo-fur" x="4" y="11" width="2" height="1" />
          <rect className="sendo-fur" x="10" y="11" width="2" height="1" />
        </g>
        <g className="sendo-feet">
          <rect className="sendo-fur" x="5" y="12" width="2" height="1" />
          <rect className="sendo-fur" x="9" y="12" width="2" height="1" />
          <rect className="sendo-fur" x="4" y="13" width="3" height="1" />
          <rect className="sendo-fur" x="9" y="13" width="3" height="1" />
        </g>
      </g>
    </svg>
  );
  if (onActivate) {
    return (
      <button
        type="button"
        className={`sendo-sprite-button ${className}`.trim()}
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
