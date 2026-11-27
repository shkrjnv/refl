import type { ReactNode } from "react";

import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { ThemeName } from "../theme/tokens";
import { IellaSprite } from "./IellaSprite";

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
  onToggleTheme: () => void;
  lessonAvailable?: boolean;
}

interface NavItem {
  route: AppRoute;
  label: string;
  glyph: "today" | "practice" | "stats" | "settings";
  disabled?: boolean;
}

function NavGlyph({ name }: { name: NavItem["glyph"] }) {
  if (name === "today") {
    return (
      <svg viewBox="0 0 20 20" aria-hidden="true" className="nav-glyph">
        <path d="M4 8h12M7 3v3M13 3v3M4 5h12v11H4z" />
      </svg>
    );
  }
  if (name === "practice") {
    return (
      <svg viewBox="0 0 20 20" aria-hidden="true" className="nav-glyph">
        <path d="M3 7l7-3 7 3-7 3zM6 9v4c2.2 2 5.8 2 8 0V9" />
      </svg>
    );
  }
  if (name === "stats") {
    return (
      <svg viewBox="0 0 20 20" aria-hidden="true" className="nav-glyph">
        <path d="M4 16V10M8.5 16V5M13 16v-8M17 16v-4" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true" className="nav-glyph">
      <path d="M10 7.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5zM10 2v3M10 15v3M2 10h3M15 10h3" />
    </svg>
  );
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
  onToggleTheme,
  lessonAvailable = false,
}: AppShellProps) {
  const { t } = useTranslation();
  const items: NavItem[] = [
    { route: "today", label: t("nav-today"), glyph: "today" },
    {
      route: "lesson",
      label: t("nav-practice"),
      glyph: "practice",
      disabled: !lessonAvailable,
    },
    { route: "stats", label: t("nav-stats"), glyph: "stats" },
    { route: "profile", label: t("nav-settings"), glyph: "settings" },
  ];

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
        <nav className="side-nav">
          {items.map((item) => (
            <button
              key={item.route}
              type="button"
              className={`nav-item${active === item.route ? " active" : ""}`}
              aria-current={active === item.route ? "page" : undefined}
              disabled={item.disabled}
              onClick={() => navigate(item)}
            >
              <NavGlyph name={item.glyph} />
              <span>{item.label}</span>
            </button>
          ))}
        </nav>
        <div className="sidebar-spacer" />
        <div className="iella-card">
          <IellaSprite size={44} />
          <div>
            <strong>Iella</strong>
            <span>{t("iella-companion")}</span>
          </div>
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
            <button
              type="button"
              className="theme-toggle"
              aria-label={t("app-theme")}
              onClick={onToggleTheme}
            >
              {theme === "light" ? t("common-dark") : t("common-light")}
            </button>
          </div>
        </header>
        <main className="studio-content">{children}</main>
      </div>

      <nav className="mobile-nav" aria-label={t("nav-label")}>
        {items.map((item) => (
          <button
            key={item.route}
            type="button"
            className={`mobile-nav-item${active === item.route ? " active" : ""}`}
            aria-current={active === item.route ? "page" : undefined}
            disabled={item.disabled}
            onClick={() => navigate(item)}
          >
            <NavGlyph name={item.glyph} />
            <span>{item.label}</span>
          </button>
        ))}
      </nav>
    </div>
  );
}
