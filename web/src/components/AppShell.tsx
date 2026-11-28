import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";

import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import { SOURCE_CODE_URL, SOURCE_LICENSE } from "../pwa/links";
import type { NavigationPosition } from "../prototype/layout";
import type { ThemeName } from "../theme/tokens";
import { CompanionSprite } from "./CompanionSprite";
import { companionById, type CompanionId } from "../companions/registry";
import { LanguageMenu } from "./LanguageMenu";
import { BrandLogo, StudioIcon, type StudioIconName } from "./StudioIcons";
import { ThemeMenu } from "./ThemeMenu";

export type AppRoute =
  | "today"
  | "vocabulary"
  | "library"
  | "graph"
  | "lesson"
  | "stats"
  | "profile"
  | "notes";

export interface AppShellProps {
  active: AppRoute;
  onNavigate: (route: AppRoute) => void;
  children: ReactNode;
  learnerLabel: string;
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  theme: ThemeName;
  onThemeChange: (theme: ThemeName) => void;
  navigationPosition: NavigationPosition;
  onIellaActivate?: () => void;
  lessonAvailable?: boolean;
  companion: CompanionId;
}

const DOCK_ROUTES: ReadonlySet<AppRoute> = new Set([
  "today",
  "vocabulary",
  "library",
  "notes",
]);

/** Below this width the dock is a phone bar; above it, a centred bar. */
const DOCK_MEDIA_QUERY = "(max-width: 820px)";

function sheetItems(container: HTMLElement | null): HTMLButtonElement[] {
  return container
    ? [...container.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")]
    : [];
}

interface NavItem {
  route: AppRoute;
  label: string;
  glyph: StudioIconName;
  disabled?: boolean;
}

export function AppShell({
  active,
  onNavigate,
  children,
  learnerLabel,
  locale,
  locales,
  onLocaleChange,
  theme,
  onThemeChange,
  navigationPosition,
  onIellaActivate,
  lessonAvailable = false,
  companion,
}: AppShellProps) {
  const companionEntry = companionById(companion);
  const { t } = useTranslation();
  // A learner without a display name still gets a neutral identity card.
  const learnerName = learnerLabel.trim() || t("profile-learner");
  const primaryItems: NavItem[] = [
    { route: "today", label: t("nav-today"), glyph: "today" },
    { route: "vocabulary", label: t("nav-vocabulary"), glyph: "vocabulary" },
    { route: "library", label: t("nav-library"), glyph: "library" },
    { route: "notes", label: t("nav-notes"), glyph: "note" },
    { route: "graph", label: t("nav-graph"), glyph: "graph" },
    {
      route: "lesson",
      label: t("nav-practice"),
      glyph: "practice",
      disabled: !lessonAvailable,
    },
    { route: "stats", label: t("nav-stats"), glyph: "progress" },
  ];
  const profileItem: NavItem = {
    route: "profile",
    label: t("nav-profile"),
    glyph: "profile",
  };

  // The phone dock holds five equal slots so every label fits at 360 px;
  // the remaining sections live in the More sheet.
  const dockItems = primaryItems.filter((item) => DOCK_ROUTES.has(item.route));
  const moreItems = [
    ...primaryItems.filter((item) => !DOCK_ROUTES.has(item.route)),
    profileItem,
  ];
  const moreActive = moreItems.some((item) => item.route === active);
  const [moreOpen, setMoreOpen] = useState(false);
  const [sheetStyle, setSheetStyle] = useState<CSSProperties | undefined>();
  const moreRef = useRef<HTMLDivElement | null>(null);
  const moreButtonRef = useRef<HTMLButtonElement | null>(null);
  const sheetRef = useRef<HTMLDivElement | null>(null);

  const closeMore = useCallback(() => {
    setMoreOpen(false);
    moreButtonRef.current?.focus();
  }, []);

  // On a phone the sheet is full-width above the dock and CSS anchors it; on
  // a wide bottom dock it is a popover above the More button, one dock step
  // wide and aligned to the button's right edge.
  useLayoutEffect(() => {
    if (!moreOpen) {
      setSheetStyle(undefined);
      return;
    }
    const place = () => {
      const matchesPhone =
        typeof window.matchMedia === "function" &&
        window.matchMedia(DOCK_MEDIA_QUERY).matches;
      if (matchesPhone) {
        setSheetStyle(undefined);
        return;
      }
      const dock = moreRef.current?.querySelector(".mobile-nav");
      const button = moreButtonRef.current;
      if (!dock || !button) {
        return;
      }
      const dockRect = dock.getBoundingClientRect();
      const buttonRect = button.getBoundingClientRect();
      const itemCount =
        dock.querySelectorAll(".mobile-nav-item").length || 1;
      const width = Math.max(buttonRect.width, dockRect.width / itemCount);
      const margin = 8;
      const right = Math.min(window.innerWidth - margin, buttonRect.right);
      setSheetStyle({
        right: "auto",
        bottom: Math.max(margin, window.innerHeight - buttonRect.top + margin),
        left: Math.max(margin, right - width),
        width,
      });
    };
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
  }, [moreOpen]);

  // Move focus into the sheet when it opens.
  useEffect(() => {
    if (!moreOpen) {
      return;
    }
    const timer = window.setTimeout(() => {
      sheetItems(sheetRef.current)[0]?.focus();
    }, 0);
    return () => window.clearTimeout(timer);
  }, [moreOpen]);

  useEffect(() => {
    if (!moreOpen) {
      return;
    }
    const onPointer = (event: PointerEvent) => {
      if (!moreRef.current?.contains(event.target as Node)) {
        closeMore();
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeMore();
      }
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [closeMore, moreOpen]);

  const onSheetKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key !== "Tab") {
      return;
    }
    const items = sheetItems(sheetRef.current);
    if (items.length === 0) {
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const current = document.activeElement;
    if (!first || !last) {
      return;
    }
    if (!sheetRef.current?.contains(current)) {
      event.preventDefault();
      first.focus();
      return;
    }
    if (event.shiftKey && current === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && current === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const navigate = (item: NavItem) => {
    if (!item.disabled) {
      closeMore();
      onNavigate(item.route);
    }
  };

  return (
    <div className="studio-shell" data-nav-position={navigationPosition}>
      <aside className="studio-sidebar" aria-label={t("nav-label")}>
        <div className="brand-lockup">
          <BrandLogo size={52} />
          <span className="brand-name">{t("app-name")}</span>
        </div>
        <nav className="side-nav" aria-label={t("nav-label")}>
          {primaryItems.map((item) => (
            <button
              key={item.route}
              type="button"
              className={`nav-item${active === item.route ? " active" : ""}`}
              aria-current={active === item.route ? "page" : undefined}
              disabled={item.disabled}
              onClick={() => navigate(item)}
            >
              <StudioIcon name={item.glyph} />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-spacer" />
        <div className="iella-card">
          <CompanionSprite
            companion={companion}
            size={36}
            className="iella-card-sprite"
          />
          <span className="iella-card-copy">
            <span className="iella-card-kicker">{t(companionEntry.kickerKey)}</span>
            <strong>{t(companionEntry.nameKey)}</strong>
          </span>
          {onIellaActivate ? (
            <button
              type="button"
              className="iella-card-action"
              aria-label={t("iella-open-tour")}
              onClick={onIellaActivate}
            >
              <StudioIcon name="spark" size={16} />
            </button>
          ) : null}
        </div>
        <div className="learner-dock">
          <button
            type="button"
            className={`learner-profile-button${active === "profile" ? " active" : ""}`}
            aria-current={active === "profile" ? "page" : undefined}
            onClick={() => onNavigate("profile")}
          >
            <span className="learner-avatar">{learnerName.slice(0, 1)}</span>
            <span className="learner-copy">
              <strong>{learnerName}</strong>
              <span>
                {t("nav-profile")} · {t("profile-local-learner")}
              </span>
            </span>
          </button>
        </div>
      </aside>

      <div className="studio-main">
        <header className="studio-topbar">
          <div className="mobile-brand">
            <BrandLogo size={32} />
            <span className="brand-name">{t("app-name")}</span>
          </div>
          <div className="topbar-actions">
            <LanguageMenu
              locale={locale}
              locales={locales}
              onLocaleChange={onLocaleChange}
            />
            <ThemeMenu theme={theme} onThemeChange={onThemeChange} />
          </div>
        </header>
        <main className="studio-content">{children}</main>
        <footer className="studio-footer">
          <span>
            refl · {SOURCE_LICENSE}
          </span>
          <a href={SOURCE_CODE_URL} target="_blank" rel="noreferrer">
            {t("app-source-link")}
          </a>
        </footer>
      </div>

      <div className="mobile-dock" ref={moreRef}>
        {moreOpen ? (
          <div
            id="mobile-more-sheet"
            ref={sheetRef}
            className="mobile-more-sheet"
            role="dialog"
            aria-modal="true"
            aria-label={t("nav-more-title")}
            style={sheetStyle}
            onKeyDown={onSheetKeyDown}
          >
            <p className="mobile-more-title">{t("nav-more-title")}</p>
            <nav aria-label={t("nav-more-title")}>
              {moreItems.map((item) => (
                <button
                  key={item.route}
                  type="button"
                  className={`mobile-more-item${active === item.route ? " active" : ""}`}
                  aria-current={active === item.route ? "page" : undefined}
                  disabled={item.disabled}
                  onClick={() => navigate(item)}
                >
                  <StudioIcon name={item.glyph} size={20} />
                  <span>{item.label}</span>
                </button>
              ))}
            </nav>
          </div>
        ) : null}
        <nav className="mobile-nav" aria-label={t("nav-label")}>
          {dockItems.map((item) => (
            <button
              key={item.route}
              type="button"
              className={`mobile-nav-item${active === item.route ? " active" : ""}`}
              aria-current={active === item.route ? "page" : undefined}
              disabled={item.disabled}
              onClick={() => navigate(item)}
            >
              <StudioIcon name={item.glyph} />
              <span>{item.label}</span>
            </button>
          ))}
          <button
            ref={moreButtonRef}
            type="button"
            className={`mobile-nav-item${moreActive || moreOpen ? " active" : ""}`}
            aria-expanded={moreOpen}
            aria-controls="mobile-more-sheet"
            aria-haspopup="true"
            onClick={() =>
              setMoreOpen((open) => {
                if (open) {
                  moreButtonRef.current?.focus();
                }
                return !open;
              })
            }
          >
            <StudioIcon name="more" />
            <span>{t("nav-more")}</span>
          </button>
        </nav>
      </div>
    </div>
  );
}
