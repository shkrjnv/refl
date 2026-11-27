import type { ReactNode } from "react";

import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import type { SkinName, ThemeName } from "../theme/tokens";
import { AppearanceMenu } from "./AppearanceMenu";
import { IellaSprite } from "./IellaSprite";
import { LanguageMenu } from "./LanguageMenu";
import { BrandLogo, StudioIcon, type StudioIconName } from "./StudioIcons";

export type AppRoute = "today" | "lesson" | "stats" | "profile";

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
  skin: SkinName;
  onSkinChange: (skin: SkinName) => void;
  navigationPosition: NavigationPosition;
  onIellaActivate?: () => void;
  lessonAvailable?: boolean;
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
  skin,
  onSkinChange,
  navigationPosition,
  onIellaActivate,
  lessonAvailable = false,
}: AppShellProps) {
  const { t } = useTranslation();
  const primaryItems: NavItem[] = [
    { route: "today", label: t("nav-today"), glyph: "today" },
    {
      route: "lesson",
      label: t("nav-practice"),
      glyph: "practice",
      disabled: !lessonAvailable,
    },
    { route: "stats", label: t("nav-stats"), glyph: "progress" },
    { route: "profile", label: t("nav-profile"), glyph: "profile" },
  ];

  const navigate = (item: NavItem) => {
    if (!item.disabled) {
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
          <IellaSprite size={36} className="iella-card-sprite" />
          <span className="iella-card-copy">
            <span className="iella-card-kicker">{t("iella-companion")}</span>
            <strong>{t("iella-name")}</strong>
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
            <span className="learner-avatar">{learnerLabel.slice(0, 1)}</span>
            <span className="learner-copy">
              <strong>{learnerLabel}</strong>
              <span>{t("profile-local-learner")}</span>
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
            <AppearanceMenu
              theme={theme}
              skin={skin}
              onThemeChange={onThemeChange}
              onSkinChange={onSkinChange}
            />
          </div>
        </header>
        <main className="studio-content">{children}</main>
      </div>

      <nav className="mobile-nav" aria-label={t("nav-label")}>
        {primaryItems.map((item) => (
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
      </nav>
    </div>
  );
}
