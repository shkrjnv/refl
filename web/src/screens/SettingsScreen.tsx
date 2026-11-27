import type { ApiClient } from "../api/client";
import { IellaSprite } from "../components/IellaSprite";
import { Badge, Button, Card } from "../components/ui";
import type { OutboxRecord } from "../db/storage";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { AppStatus } from "../prototype/status";
import {
  skinDescriptionKeys,
  skinLabelKeys,
  skinNames,
  type SkinName,
  type ThemeName,
} from "../theme/tokens";
import { AdminAccessSection } from "./AdminAccessSection";

export interface SettingsScreenProps {
  learnerLabel: string;
  authMethod: "email" | "telegram";
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  theme: ThemeName;
  onThemeChange: (theme: ThemeName) => void;
  skin: SkinName;
  onSkinChange: (skin: SkinName) => void;
  onReplayTour: () => void;
  status: AppStatus;
  rejected: OutboxRecord[];
  onLogout: () => void;
  isAdmin: boolean;
  adminApi: ApiClient | null;
}

function persistenceKey(status: AppStatus): string {
  switch (status.persistence) {
    case "granted":
      return "storage-persist-granted";
    case "denied":
      return "storage-persist-denied";
    case "unsupported":
      return "storage-persist-unsupported";
    default:
      return "storage-persist-unknown";
  }
}

export function SettingsScreen({
  learnerLabel,
  authMethod,
  locale,
  locales,
  onLocaleChange,
  theme,
  onThemeChange,
  skin,
  onSkinChange,
  onReplayTour,
  status,
  rejected,
  onLogout,
  isAdmin,
  adminApi,
}: SettingsScreenProps) {
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
        <Card tone="plain" className="settings-card settings-appearance">
          <p className="eyebrow">{t("profile-appearance")}</p>
          <h2>{t("profile-skin")}</h2>
          <p className="settings-note">{t("profile-skin-note")}</p>
          <div className="skin-options">
            {skinNames.map((candidate) => (
              <button
                key={candidate}
                type="button"
                className={`skin-option${skin === candidate ? " selected" : ""}`}
                aria-pressed={skin === candidate}
                onClick={() => onSkinChange(candidate)}
              >
                <span
                  className={`skin-swatch skin-swatch-${candidate}`}
                  aria-hidden="true"
                />
                <span className="skin-option-copy">
                  <strong>{t(skinLabelKeys[candidate])}</strong>
                  <span>{t(skinDescriptionKeys[candidate])}</span>
                </span>
              </button>
            ))}
          </div>
          <p className="eyebrow appearance-mode-label">{t("profile-mode")}</p>
          <div className="settings-options">
            <Button
              type="button"
              variant={theme === "light" ? "primary" : "secondary"}
              size="sm"
              aria-pressed={theme === "light"}
              onClick={() => onThemeChange("light")}
            >
              {t("common-light")}
            </Button>
            <Button
              type="button"
              variant={theme === "dark" ? "primary" : "secondary"}
              size="sm"
              aria-pressed={theme === "dark"}
              onClick={() => onThemeChange("dark")}
            >
              {t("common-dark")}
            </Button>
          </div>
        </Card>

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
        <p className={`storage-status storage-${status.persistence}`}>
          {t(persistenceKey(status))}
        </p>
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
