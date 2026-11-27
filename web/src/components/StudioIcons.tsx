import { useId, type ReactNode } from "react";

export type StudioIconName =
  | "today"
  | "practice"
  | "progress"
  | "settings"
  | "gear"
  | "profile"
  | "sync"
  | "offline"
  | "palette"
  | "check"
  | "close"
  | "arrow-left"
  | "arrow-right"
  | "arrow-up"
  | "arrow-down"
  | "chevron-down"
  | "grip"
  | "streak"
  | "spark"
  | "sun"
  | "moon"
  | "globe";

export interface StudioIconProps {
  name: StudioIconName;
  size?: number;
  className?: string;
  strokeWidth?: number;
}

function paths(name: StudioIconName): ReactNode {
  switch (name) {
    case "today":
      return (
        <>
          <path d="M3.5 18h17" />
          <path d="M7 18a5 5 0 0 1 10 0" />
          <circle cx="12" cy="9" r="3.1" />
          <path d="M12 3.2v1.7M5.8 5.4 7 6.6M18.2 5.4 17 6.6M3.4 11.5h1.7M18.9 11.5h1.7" />
        </>
      );
    case "practice":
      return (
        <>
          <path d="M4 6.1c2.7-.8 5.4-.5 8 .9 2.6-1.4 5.3-1.7 8-.9v11.5c-2.7-.8-5.4-.5-8 .9-2.6-1.4-5.3-1.7-8-.9z" />
          <path d="M12 7v11.5" />
          <path d="m17.4 2.8.6 1.6 1.6.6-1.6.6-.6 1.6-.6-1.6-1.6-.6 1.6-.6z" />
        </>
      );
    case "progress":
      return (
        <>
          <rect x="3.6" y="14" width="3.4" height="5.6" rx="1" />
          <rect x="10.3" y="10.2" width="3.4" height="9.4" rx="1" />
          <rect x="17" y="6.4" width="3.4" height="13.2" rx="1" />
          <path d="m4.4 9.8 4.5-3.1 3.2 1.6 7.1-5.2" />
          <path d="M16.5 3.1h2.9v2.9" />
        </>
      );
    case "settings":
      return (
        <>
          <path d="M4 7h7.2M17.5 7H20M4 17h2.5M11.8 17H20" />
          <circle cx="14.2" cy="7" r="2.2" />
          <circle cx="8.2" cy="17" r="2.2" />
        </>
      );
    case "gear":
      return (
        <>
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.38a2 2 0 0 0-.73-2.73l-.15-.09a2 2 0 0 1-1-1.74v-.51a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
          <circle cx="12" cy="12" r="3" />
        </>
      );
    case "profile":
      return (
        <>
          <circle cx="12" cy="7.8" r="3.4" />
          <path d="M4.5 19.6c.1-4 3.4-6.7 7.5-6.7s7.4 2.7 7.5 6.7" />
          <path d="M4.4 19.6h15.2" />
        </>
      );
    case "sync":
      return (
        <>
          <path d="M5.5 9.5A7 7 0 0 1 18 7.3L20 9" />
          <path d="M20 5v4h-4" />
          <path d="M18.5 14.5A7 7 0 0 1 6 16.7L4 15" />
          <path d="M4 19v-4h4" />
        </>
      );
    case "offline":
      return (
        <>
          <path d="M6.5 17.5h11a3.5 3.5 0 0 0 .6-6.9A5.8 5.8 0 0 0 7.4 9.1" />
          <path d="M6.5 17.5a3.5 3.5 0 0 1-.6-6.9" />
          <path d="m4 4 16 16" />
        </>
      );
    case "palette":
      return (
        <>
          <path d="M12 3.5a8.5 8.5 0 1 0 0 17h1.1a1.8 1.8 0 0 0 1.2-3.1 1.8 1.8 0 0 1 1.2-3.1H18a2.5 2.5 0 0 0 2.5-2.5A8.5 8.5 0 0 0 12 3.5Z" />
          <circle cx="8" cy="9" r="0.7" />
          <circle cx="12" cy="7" r="0.7" />
          <circle cx="16" cy="9" r="0.7" />
        </>
      );
    case "check":
      return <path d="m4.5 12.5 5 5L20 6.8" />;
    case "close":
      return <path d="M6 6l12 12M18 6 6 18" />;
    case "arrow-left":
      return (
        <>
          <path d="M19 12H5" />
          <path d="m11 6-6 6 6 6" />
        </>
      );
    case "arrow-right":
      return (
        <>
          <path d="M5 12h14" />
          <path d="m13 6 6 6-6 6" />
        </>
      );
    case "arrow-up":
      return (
        <>
          <path d="M12 19V5" />
          <path d="m6 11 6-6 6 6" />
        </>
      );
    case "arrow-down":
      return (
        <>
          <path d="M12 5v14" />
          <path d="m6 13 6 6 6-6" />
        </>
      );
    case "grip":
      return (
        <>
          <circle cx="9" cy="6" r="0.9" />
          <circle cx="15" cy="6" r="0.9" />
          <circle cx="9" cy="12" r="0.9" />
          <circle cx="15" cy="12" r="0.9" />
          <circle cx="9" cy="18" r="0.9" />
          <circle cx="15" cy="18" r="0.9" />
        </>
      );
    case "chevron-down":
      return <path d="m6 9 6 6 6-6" />;
    case "streak":
      return (
        <>
          <path d="M13 3.5c.4 3-1.2 4.1-2.5 5.4-1.5 1.4-2.5 2.7-2.5 4.7a4.4 4.4 0 0 0 8.8 0c0-2.5-1.4-3.8-2.5-5.1-.9-1-1.5-2-1.3-5Z" />
          <path d="M12 20.5c-3.3 0-5.8-2.4-5.8-5.5 0-1.5.6-2.8 1.6-4" />
        </>
      );
    case "spark":
      return (
        <>
          <path d="M12 3.5c.7 4.9 2.1 6.3 7 7-4.9.7-6.3 2.1-7 7-.7-4.9-2.1-6.3-7-7 4.9-.7 6.3-2.1 7-7Z" />
          <path d="M18.5 17.5c.2 1.6.7 2.1 2.3 2.3-1.6.2-2.1.7-2.3 2.3-.2-1.6-.7-2.1-2.3-2.3 1.6-.2 2.1-.7 2.3-2.3Z" />
        </>
      );
    case "sun":
      return (
        <>
          <circle cx="12" cy="12" r="3.4" />
          <path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M18.7 5.3l-1.4 1.4M6.7 17.3l-1.4 1.4" />
        </>
      );
    case "moon":
      return (
        <>
          <path d="M19.5 15.2A8.2 8.2 0 0 1 8.8 4.5 8.2 8.2 0 1 0 19.5 15.2Z" />
          <path d="M16.5 4.5v2M15.5 5.5h2" />
        </>
      );
    case "globe":
      return (
        <>
          <circle cx="12" cy="12" r="8.5" />
          <path d="M3.8 9.5h16.4M3.8 14.5h16.4" />
          <path d="M12 3.5c2.2 2.4 3.3 5.3 3.3 8.5s-1.1 6.1-3.3 8.5c-2.2-2.4-3.3-5.3-3.3-8.5s1.1-6.1 3.3-8.5Z" />
        </>
      );
    default:
      return null;
  }
}

export interface BrandLogoProps {
  size?: number;
  className?: string;
}

// Two interlocking prism brackets: the left r stem and shoulder weave over the
// right stem, while the mirrored blue/teal r returns the reflection.
export function BrandLogo({ size = 28, className = "" }: BrandLogoProps) {
  const uid = useId().replace(/[^a-zA-Z0-9_-]/g, "");
  const umber = `brand-umber-${uid}`;
  const cream = `brand-cream-${uid}`;
  const blue = `brand-blue-${uid}`;
  const blueLight = `brand-blue-light-${uid}`;
  const teal = `brand-teal-${uid}`;
  return (
    <svg
      className={`brand-logo ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 40 40"
      aria-hidden="true"
      focusable="false"
      data-mark="prism-reflection"
    >
      <defs>
        <linearGradient id={umber} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#6B4632" />
          <stop offset=".45" stopColor="#33231B" />
          <stop offset="1" stopColor="#1A1512" />
        </linearGradient>
        <linearGradient id={cream} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#FFF9EC" />
          <stop offset=".55" stopColor="#EAE0CF" />
          <stop offset="1" stopColor="#C7B49A" />
        </linearGradient>
        <linearGradient id={blue} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#63C5FF" />
          <stop offset=".5" stopColor="#1E88E5" />
          <stop offset="1" stopColor="#0B5AA6" />
        </linearGradient>
        <linearGradient id={blueLight} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#8AD5FF" />
          <stop offset="1" stopColor="#2B8FE0" />
        </linearGradient>
        <linearGradient id={teal} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#2CB8AF" />
          <stop offset=".6" stopColor="#0D7A78" />
          <stop offset="1" stopColor="#07504F" />
        </linearGradient>
      </defs>
      <g className="brand-prism">
        {/* Left bracket: Deep Umber stem and cream shoulder crossing the blue stem. */}
        <path
          className="brand-prism-stem brand-prism-stem-umber"
          d="M7 5L12 3.4L12 33L7 35Z"
          fill={`url(#${umber})`}
        />
        <path
          className="brand-prism-facet"
          d="M7 5L9 4.4L9 34L7 35Z"
          fill="#8A5D43"
          opacity=".55"
        />
        <path
          className="brand-prism-arm brand-prism-arm-cream"
          d="M11.5 9.5L31 9.5L34 13L31 16.5L11.5 16.5Z"
          fill={`url(#${cream})`}
        />
        <path
          className="brand-prism-fold"
          d="M11.5 14.2L31 14.2L33 15.4L31 16.5L11.5 16.5Z"
          fill="#B9A488"
          opacity=".78"
        />
        <path
          className="brand-prism-highlight"
          d="M11.5 9.5L31 9.5L33.2 11.5L13 11.5Z"
          fill="#FFFDF5"
          opacity=".58"
        />
        {/* Right bracket: Morpho Blue stem and Riso Teal shoulder crossing the umber stem. */}
        <path
          className="brand-prism-stem brand-prism-stem-blue"
          d="M33 9L28 7.4L28 37L33 39Z"
          fill={`url(#${blue})`}
        />
        <path
          className="brand-prism-facet"
          d="M33 9L31 8.4L31 38L33 39Z"
          fill={`url(#${blueLight})`}
          opacity=".42"
        />
        <path
          className="brand-prism-arm brand-prism-arm-teal"
          d="M28.5 21.5L9 21.5L6 25L9 28.5L28.5 28.5Z"
          fill={`url(#${teal})`}
        />
        <path
          className="brand-prism-fold"
          d="M28.5 26.2L9 26.2L7 27.4L9 28.5L28.5 28.5Z"
          fill="#064B49"
          opacity=".8"
        />
        <path
          className="brand-prism-highlight"
          d="M28.5 21.5L9 21.5L6.8 23.5L27 23.5Z"
          fill="#44CDC4"
          opacity=".28"
        />
        {/* Over-under shadows make the two brackets read as one woven mark. */}
        <path
          className="brand-prism-shadow"
          d="M29 9.5L34 9.5L34 16.5L31 16.5Z"
          fill="#063E6F"
          opacity=".35"
        />
        <path
          className="brand-prism-shadow"
          d="M6 21.5L11 21.5L11 28.5L9 28.5Z"
          fill="#0B0705"
          opacity=".4"
        />
        <path
          className="brand-prism-bevel"
          d="M11.5 9.5L14 12L11.5 14.5Z"
          fill="#FFFDF5"
          opacity=".22"
        />
        <path
          className="brand-prism-bevel"
          d="M28.5 21.5L26 24L28.5 26.5Z"
          fill="#75E4DC"
          opacity=".18"
        />
      </g>
    </svg>
  );
}

export function StudioIcon({
  name,
  size = 20,
  className = "",
  strokeWidth = 1.75,
}: StudioIconProps) {
  return (
    <svg
      className={`studio-icon ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {paths(name)}
    </svg>
  );
}
