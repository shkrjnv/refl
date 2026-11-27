import type { ReactNode } from "react";

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
          <path d="M3.5 19.5h17" />
          <path d="M5.2 19.5V5M5.2 19.5h15.3" />
          <rect x="7.4" y="13" width="2.7" height="4.3" rx=".7" />
          <rect x="11.4" y="10" width="2.7" height="7.3" rx=".7" />
          <rect x="15.4" y="6.8" width="2.7" height="10.5" rx=".7" />
          <path d="m6.2 10.4 3.5-2.7 2.7 1.6 5-5" />
          <path d="M15.7 4.2h1.8V6" />
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

export function BrandLogo({ size = 28, className = "" }: BrandLogoProps) {
  return (
    <svg
      className={`brand-logo ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 28 28"
      aria-hidden="true"
      focusable="false"
    >
      <rect className="brand-logo-field" x="0.8" y="0.8" width="26.4" height="26.4" rx="8.2" />
      <path className="brand-logo-fold" d="M7.1 10.1 14 6.1l6.9 4v3.2L14 17.4l-6.9-4.1z" />
      <path className="brand-logo-reflect" d="m7.1 15.2 6.9 4.1 6.9-4.1v3.1L14 22.5l-6.9-4.2z" />
      <path className="brand-logo-spine" d="M14 6.1v16.4" />
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
