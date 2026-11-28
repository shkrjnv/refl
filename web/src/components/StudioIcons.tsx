import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  BarChart3,
  BookMarked,
  BookOpen,
  Bookmark,
  Check,
  ChevronDown,
  ChevronUp,
  Cog,
  Copy,
  Flame,
  Globe,
  GripVertical,
  LibraryBig,
  Minus,
  Moon,
  Network,
  NotebookPen,
  Palette,
  Plus,
  RefreshCw,
  Settings,
  Sparkles,
  Sun,
  UserRound,
  WifiOff,
  X,
  type LucideIcon,
} from "lucide-react";

export type StudioIconName =
  | "today"
  | "vocabulary"
  | "library"
  | "graph"
  | "bookmark"
  | "note"
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
  | "globe"
  | "copy"
  | "plus"
  | "minus"
  | "chevron-up";

export interface StudioIconProps {
  name: StudioIconName;
  size?: number;
  className?: string;
  strokeWidth?: number;
}

const ICONS: Record<StudioIconName, LucideIcon> = {
  today: Sun,
  vocabulary: BookMarked,
  library: LibraryBig,
  graph: Network,
  bookmark: Bookmark,
  note: NotebookPen,
  practice: BookOpen,
  progress: BarChart3,
  settings: Settings,
  gear: Cog,
  profile: UserRound,
  sync: RefreshCw,
  offline: WifiOff,
  palette: Palette,
  check: Check,
  close: X,
  "arrow-left": ArrowLeft,
  "arrow-right": ArrowRight,
  "arrow-up": ArrowUp,
  "arrow-down": ArrowDown,
  "chevron-down": ChevronDown,
  grip: GripVertical,
  streak: Flame,
  spark: Sparkles,
  sun: Sun,
  moon: Moon,
  globe: Globe,
  copy: Copy,
  plus: Plus,
  minus: Minus,
  "chevron-up": ChevronUp,
};

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
      fill="none"
      aria-hidden="true"
      focusable="false"
      data-mark="reflection"
    >
      <path
        className="brand-reflection-ink"
        d="M6.5 22V13C6.5 8.5 10 5 14.5 5L16 5"
        stroke="currentColor"
        strokeWidth="2.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        className="brand-reflection-accent"
        d="M21.5 6V15C21.5 19.5 18 23 13.5 23L12 23"
        stroke="var(--accent-primary)"
        strokeWidth="2.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function StudioIcon({
  name,
  size = 20,
  className = "",
  strokeWidth = 1.75,
}: StudioIconProps) {
  const Icon = ICONS[name];
  return (
    <Icon
      className={`studio-icon ${className}`.trim()}
      size={size}
      strokeWidth={strokeWidth}
      aria-hidden="true"
      focusable="false"
    />
  );
}

export interface CrossfadeIconProps {
  name: StudioIconName;
  activeName?: StudioIconName;
  active?: boolean;
  size?: number;
  className?: string;
}

export function CrossfadeIcon({
  name,
  activeName = name,
  active = false,
  size = 20,
  className = "",
}: CrossfadeIconProps) {
  return (
    <span
      className={`crossfade-icon ${className}`.trim()}
      data-active={active ? "true" : "false"}
      aria-hidden="true"
      style={{ width: size, height: size }}
    >
      <span className="crossfade-icon-state crossfade-icon-state-from">
        <StudioIcon name={name} size={size} />
      </span>
      <span className="crossfade-icon-state crossfade-icon-state-to">
        <StudioIcon name={activeName} size={size} />
      </span>
    </span>
  );
}

export interface ConfirmationCheckProps {
  size?: number;
  className?: string;
}

/**
 * A check drawn by CSS stroke-dashoffset rather than a static glyph. The
 * wrapper controls colour and layout; the path carries the draw-in animation.
 */
export function ConfirmationCheck({
  size = 20,
  className = "",
}: ConfirmationCheckProps) {
  return (
    <svg
      className={`confirmation-check ${className}`.trim()}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
      focusable="false"
    >
      <path
        className="confirmation-check-path"
        d="m4.5 12.5 5 5L20 6.8"
        pathLength={1}
        strokeDasharray={1}
        strokeDashoffset={1}
        stroke="currentColor"
        strokeWidth={2.4}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
