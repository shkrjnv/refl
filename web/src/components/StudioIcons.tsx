import type { ReactNode } from "react";

export type StudioIconName =
  | "today"
  | "practice"
  | "progress"
  | "settings"
  | "sync"
  | "offline"
  | "palette"
  | "check"
  | "close"
  | "arrow-left"
  | "arrow-right"
  | "chevron-down"
  | "streak"
  | "spark"
  | "sun"
  | "moon";

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
          <circle cx="12" cy="11" r="3.2" />
          <path d="M4 18h16" />
          <path d="M12 3v2M5.6 5.6l1.4 1.4M18.4 5.6 17 7" />
          <path d="M6.3 14.8a6.3 6.3 0 0 1 11.4 0" />
        </>
      );
    case "practice":
      return (
        <>
          <path d="M4 8.2 12 4l8 4.2-8 4.2z" />
          <path d="M7 10.2v4.1c2.9 2.4 7.1 2.4 10 0v-4.1" />
          <path d="M20 8.2v5.6" />
        </>
      );
    case "progress":
      return (
        <>
          <path d="M4 19h16" />
          <path d="M5.5 16.2c3.2 0 3.2-3.1 6.3-3.1s3.2-4.1 6.3-4.1" />
          <circle cx="5.5" cy="16.2" r="1.35" />
          <circle cx="11.8" cy="13.1" r="1.35" />
          <circle cx="18.1" cy="9" r="1.35" />
        </>
      );
    case "settings":
      return (
        <>
          <path d="M4 7h5M13 7h7M4 17h7M15 17h5" />
          <circle cx="11" cy="7" r="2" />
          <circle cx="13" cy="17" r="2" />
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
    default:
      return null;
  }
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
