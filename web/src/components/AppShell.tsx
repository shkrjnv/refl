import type { ReactNode } from "react";

import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { SkinName, ThemeName } from "../theme/tokens";
import { AppearanceMenu } from "./AppearanceMenu";
import { IellaCompanion } from "./IellaCompanion";
import type { IellaMood } from "./IellaSprite";
import { StudioIcon, type StudioIconName } from "./StudioIcons";

export type AppRoute = "today" | "lesson" | "stats" | "settings";

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
  onIellaActivate?: () => void;
  lessonAvailable?: boolean;
  iellaMood?: IellaMood;
  iellaSignal?: number;
  onIellaReactive?: () => void;
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
  onIellaActivate,
  lessonAvailable = false,
  iellaMood = "idle",
  iellaSignal = 0,
  onIellaReactive,
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
  ];
  const settingsItem: NavItem = {
    route: "settings",
    label: t("nav-settings"),
    glyph: "settings",
  };

  const navigate = (item: NavItem) => {
    if (!item.disabled) {
      onNavigate(item.route);
    }
  };

  return (
    <div className="studio-shell">
      <aside className="studio-sidebar" aria-label={t("nav-label")}>
        <div className="brand-lockup">
          <span className="brand-mark" aria-hidden="true">
            r
          </span>
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
        <div className="side-nav-utility">
          <div className="nav-divider" aria-hidden="true" />
          <button
            type="button"
            className={`nav-item${active === settingsItem.route ? " active" : ""}`}
            aria-current={active === settingsItem.route ? "page" : undefined}
            onClick={() => navigate(settingsItem)}
          >
            <StudioIcon name={settingsItem.glyph} />
            <span>{settingsItem.label}</span>
          </button>
        </div>
        <div
          className="iella-card"
          data-iella-perch="sidebar"
          data-iella-route="app"
        >
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
        <div className="learner-chip">
          <span className="learner-avatar">{learnerLabel.slice(0, 1)}</span>
          <span className="learner-copy">
            <strong>{learnerLabel}</strong>
            <span>{t("profile-local-learner")}</span>
          </span>
        </div>
      </aside>

      <div className="studio-main">
        <header className="studio-topbar">
          <div className="mobile-brand">
            <span
              className="iella-anchor iella-anchor-mobile"
              data-iella-perch="mobile-brand"
              data-iella-route="app"
              aria-hidden="true"
            />
            <span className="brand-mark" aria-hidden="true">
              r
            </span>
            <span className="brand-name">{t("app-name")}</span>
          </div>
          <div className="topbar-actions">
            <div className="segmented" aria-label={t("app-language")}>
              {locales.map((candidate) => (
                <button
                  key={candidate}
                  type="button"
                  className={
                    candidate === locale ? "segment active" : "segment"
                  }
                  aria-pressed={candidate === locale}
                  onClick={() => onLocaleChange(candidate)}
                >
                  {t(`language-${candidate}`)}
                </button>
              ))}
            </div>
            <AppearanceMenu
              theme={theme}
              skin={skin}
              onThemeChange={(next) => {
                if (next !== theme) {
                  onIellaReactive?.();
                }
                onThemeChange(next);
              }}
              onSkinChange={(next) => {
                if (next !== skin) {
                  onIellaReactive?.();
                }
                onSkinChange(next);
              }}
            />
          </div>
        </header>
        <main className="studio-content">{children}</main>
      </div>

      <nav className="mobile-nav" aria-label={t("nav-label")}>
        {[...primaryItems, settingsItem].map((item) => (
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
      <IellaCompanion
        activeRoute={active}
        mood={iellaMood}
        signal={iellaSignal}
        onActivate={onIellaActivate}
      />
    </div>
  );
}
