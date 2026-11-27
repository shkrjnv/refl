import type { ApiClient } from "../api/client";
import { IellaSprite } from "../components/IellaSprite";
import { Badge, Button, Card } from "../components/ui";
import type { OutboxRecord } from "../db/storage";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { AppStatus } from "../prototype/status";
import type { ThemeName } from "../theme/tokens";
import { AdminAccessSection } from "./AdminAccessSection";

export interface ProfileScreenProps {
  learnerLabel: string;
  authMethod: "email" | "telegram";
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  theme: ThemeName;
  onToggleTheme: () => void;
  onReplayTour: () => void;
  status: AppStatus;
  rejected: OutboxRecord[];
  onLogout: () => void;
  isAdmin: boolean;
  adminApi: ApiClient | null;
}

export function ProfileScreen({
  learnerLabel,
  authMethod,
  locale,
  locales,
  onLocaleChange,
  theme,
  onToggleTheme,
  onReplayTour,
  status,
  rejected,
  onLogout,
  isAdmin,
  adminApi,
}: ProfileScreenProps) {
  const { t } = useTranslation();
  return (
    <div className="profile-screen">
      <header className="page-heading">
        <div>
          <p className="eyebrow">{t("profile-eyebrow")}</p>
          <h1>{t("profile-title")}</h1>
        </div>
        <Badge tone="green">{t("profile-local")}</Badge>
      </header>

      <Card tone="hero" raised className="profile-hero">
        <div className="profile-avatar">
          <IellaSprite size={82} />
        </div>
        <div className="profile-copy">
          <p className="eyebrow">{t("profile-learner")}</p>
          <h2>{learnerLabel}</h2>
          <p>{t("profile-body")}</p>
        </div>
        <div className="profile-auth">
          <span>{t("profile-linked")}</span>
          <strong>
            {authMethod === "telegram"
              ? t("auth-telegram")
              : t("auth-email-label")}
          </strong>
        </div>
      </Card>

      <div className="settings-grid">
        <Card tone="plain" className="settings-card">
          <p className="eyebrow">{t("profile-language")}</p>
          <h2>{t("app-language")}</h2>
          <div className="settings-options">
            {locales.map((candidate) => (
              <Button
                key={candidate}
                type="button"
                variant={candidate === locale ? "primary" : "secondary"}
                size="sm"
                onClick={() => onLocaleChange(candidate)}
              >
                {t(`language-${candidate}`)}
              </Button>
            ))}
          </div>
        </Card>

        <Card tone="plain" className="settings-card">
          <p className="eyebrow">{t("profile-theme")}</p>
          <h2>{t("app-theme")}</h2>
          <div className="settings-options">
            <Button
              type="button"
              variant={theme === "light" ? "primary" : "secondary"}
              size="sm"
              onClick={() => {
                if (theme !== "light") {
                  onToggleTheme();
                }
              }}
            >
              {t("common-light")}
            </Button>
            <Button
              type="button"
              variant={theme === "dark" ? "primary" : "secondary"}
              size="sm"
              onClick={() => {
                if (theme !== "dark") {
                  onToggleTheme();
                }
              }}
            >
              {t("common-dark")}
            </Button>
          </div>
        </Card>

        <Card tone="plain" className="settings-card">
          <p className="eyebrow">{t("profile-onboarding")}</p>
          <h2>{t("profile-tour-title")}</h2>
          <p className="settings-note">{t("profile-tour-body")}</p>
          <Button type="button" variant="secondary" size="sm" onClick={onReplayTour}>
            {t("profile-replay-tour")}
          </Button>
        </Card>
      </div>

      <Card tone="plain" className="settings-card diagnostics-card">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("profile-diagnostics")}</p>
            <h2>{t("profile-sync")}</h2>
          </div>
          <Badge tone={status.online ? "green" : "sun"}>
            {status.online ? t("sync-online") : t("sync-offline")}
          </Badge>
        </div>
        <div className="diagnostics-grid">
          <span>{t("sync-pending", { count: status.pendingOutbox })}</span>
          <span>{t("sync-rejected", { count: status.rejectedOutbox })}</span>
          {status.clockDrift ? <span>{t("sync-clock-warning")}</span> : null}
        </div>
        {rejected.length > 0 ? (
          <div className="rejected-list">
            <p className="eyebrow">{t("profile-rejected-events")}</p>
            {rejected.map((event) => (
              <div key={event.event_id} className="rejected-row">
                <code>{event.event_id}</code>
                <span>{event.rejected_reason ?? t("common-unknown-error")}</span>
              </div>
            ))}
          </div>
        ) : null}
      </Card>

      {isAdmin && adminApi ? <AdminAccessSection api={adminApi} /> : null}

      <Card tone="plain" className="settings-card logout-card">
        <div>
          <p className="eyebrow">{t("profile-session")}</p>
          <h2>{t("auth-sign-out")}</h2>
          <p className="settings-note">{t("profile-logout-note")}</p>
        </div>
        <Button type="button" variant="danger" onClick={onLogout}>
          {t("auth-sign-out")}
        </Button>
      </Card>
    </div>
  );
}
