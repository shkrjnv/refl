import { useState } from "react";

import type { ApiClient } from "../api/client";
import { StudioIcon, type StudioIconName } from "../components/StudioIcons";
import { Button, Card, StatusMeta } from "../components/ui";
import type { OutboxRecord } from "../db/storage";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import type { AppStatus } from "../prototype/status";
import {
  paletteLabelKeys,
  paletteNames,
  paletteNoteKeys,
  skinDescriptionKeys,
  skinLabelKeys,
  skinNames,
  type PaletteName,
  type SkinName,
} from "../theme/tokens";
import { AdminAccessSection } from "./AdminAccessSection";

export type SettingsTab = "appearance" | "account" | "offline" | "admin";

export interface SettingsScreenProps {
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  theme: PaletteName;
  onThemeChange: (theme: PaletteName) => void;
  skin: SkinName;
  onSkinChange: (skin: SkinName) => void;
  navigationPosition: NavigationPosition;
  onNavigationPositionChange: (position: NavigationPosition) => void;
  status: AppStatus;
  rejected: OutboxRecord[];
  onExportCache: () => Promise<void>;
  onResetCache: () => Promise<void>;
  isAdmin: boolean;
  adminApi: ApiClient | null;
  embedded?: boolean;
  authMethod?: "email" | "telegram";
  learnerId?: string;
  onLogout?: () => void;
}

interface SettingsTabItem {
  id: SettingsTab;
  labelKey: string;
  icon: StudioIconName;
}

interface NavigationOption {
  id: NavigationPosition;
  labelKey: string;
  icon: StudioIconName;
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
  locale,
  locales,
  onLocaleChange,
  theme,
  onThemeChange,
  skin,
  onSkinChange,
  navigationPosition,
  onNavigationPositionChange,
  status,
  rejected,
  onExportCache,
  onResetCache,
  isAdmin,
  adminApi,
  embedded = false,
  authMethod,
  learnerId,
  onLogout,
}: SettingsScreenProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<SettingsTab>("appearance");
  const [resetArmed, setResetArmed] = useState(false);
  const [dataNotice, setDataNotice] = useState<string | null>(null);
  const [dataError, setDataError] = useState<string | null>(null);
  const tabs: SettingsTabItem[] = [
    { id: "appearance", labelKey: "settings-tab-appearance", icon: "palette" },
    { id: "account", labelKey: "settings-tab-account", icon: "profile" },
    { id: "offline", labelKey: "settings-tab-offline", icon: "offline" },
    ...(isAdmin
      ? [
          {
            id: "admin" as const,
            labelKey: "settings-tab-admin",
            icon: "settings" as const,
          },
        ]
      : []),
  ];
  const navigationOptions: NavigationOption[] = [
    {
      id: "left",
      labelKey: "settings-navigation-left",
      icon: "arrow-left",
    },
    {
      id: "right",
      labelKey: "settings-navigation-right",
      icon: "arrow-right",
    },
    {
      id: "bottom",
      labelKey: "settings-navigation-bottom",
      icon: "arrow-down",
    },
  ];

  const exportCache = async () => {
    setDataNotice(null);
    setDataError(null);
    try {
      await onExportCache();
      setDataNotice(t("settings-exported"));
    } catch {
      setDataError(t("settings-export-failed"));
    }
  };

  const resetCache = async () => {
    setDataNotice(null);
    setDataError(null);
    try {
      await onResetCache();
    } catch {
      setDataError(t("settings-reset-failed"));
    }
  };

  const chooseTab = (next: SettingsTab) => {
    setTab(next);
    setResetArmed(false);
    setDataNotice(null);
    setDataError(null);
  };

  const methodRow = (kind: "email" | "telegram", labelKey: string) => {
    const linked = authMethod === kind;
    return (
      <div className="method-row" key={kind}>
        <span>{t(labelKey)}</span>
        <StatusMeta tone={linked ? "success" : "neutral"} dot>
          {linked ? t("settings-linked") : t("settings-not-linked")}
        </StatusMeta>
      </div>
    );
  };

  return (
    <div
      className={`profile-screen settings-screen${embedded ? " settings-screen-embedded" : ""}`}
    >
      {embedded ? (
        <header className="page-heading settings-embedded-heading">
          <div>
            <p className="eyebrow">{t("settings-eyebrow")}</p>
            <h1>{t("settings-title")}</h1>
            <p>{t("settings-embedded-note")}</p>
          </div>
        </header>
      ) : (
        <header className="page-heading">
          <div>
            <p className="eyebrow">{t("settings-eyebrow")}</p>
            <h1>{t("settings-title")}</h1>
          </div>
          <StatusMeta tone="neutral" dot>{t("profile-local")}</StatusMeta>
        </header>
      )}

      <div className="settings-tabs" role="tablist" aria-label={t("settings-tabs-label")}>
        {tabs.map((item) => (
          <button
            key={item.id}
            type="button"
            role="tab"
            id={`settings-tab-${item.id}`}
            aria-selected={tab === item.id}
            aria-controls={`settings-panel-${item.id}`}
            className={`settings-tab${tab === item.id ? " active" : ""}`}
            onClick={() => chooseTab(item.id)}
          >
            <StudioIcon name={item.icon} size={17} />
            <span>{t(item.labelKey)}</span>
          </button>
        ))}
      </div>

      {tab === "appearance" ? (
        <div
          className="settings-panel"
          id="settings-panel-appearance"
          role="tabpanel"
          aria-labelledby="settings-tab-appearance"
        >
          <div className="settings-panel-heading">
            <div>
              <h2>{t("profile-appearance")}</h2>
              <p>{t("profile-palette-note")}</p>
            </div>
          </div>

          <div className="palette-options" role="group" aria-label={t("app-palette")}>
            {paletteNames.map((candidate) => (
              <button
                key={candidate}
                type="button"
                className={`palette-option${theme === candidate ? " selected" : ""}`}
                aria-pressed={theme === candidate}
                onClick={() => onThemeChange(candidate)}
              >
                <span
                  className={`palette-preview palette-preview-${candidate}`}
                  aria-hidden="true"
                >
                  <span />
                  <span />
                </span>
                <span className="skin-option-copy">
                  <strong>{t(paletteLabelKeys[candidate])}</strong>
                  <span>{t(paletteNoteKeys[candidate])}</span>
                </span>
                {theme === candidate ? (
                  <StudioIcon name="check" size={16} className="appearance-check" />
                ) : null}
              </button>
            ))}
          </div>

          <div className="settings-panel-heading settings-panel-heading-spaced">
            <div>
              <h2>{t("profile-skin")}</h2>
              <p>{t("profile-skin-note")}</p>
            </div>
          </div>
          <div className="skin-options skin-options-wide">
            {skinNames.map((candidate) => (
              <button
                key={candidate}
                type="button"
                className={`skin-option${skin === candidate ? " selected" : ""}`}
                aria-pressed={skin === candidate}
                onClick={() => onSkinChange(candidate)}
              >
                <span
                  className={`skin-preview skin-preview-${candidate}`}
                  aria-hidden="true"
                >
                  <span />
                  <span />
                </span>
                <span className="skin-option-copy">
                  <strong>{t(skinLabelKeys[candidate])}</strong>
                  <span>{t(skinDescriptionKeys[candidate])}</span>
                </span>
              </button>
            ))}
          </div>

          <Card tone="plain" className="settings-card">
            <h2>{t("settings-navigation-title")}</h2>
            <p className="settings-note">{t("settings-navigation-note")}</p>
            <div className="navigation-options" role="group" aria-label={t("settings-navigation-title")}>
              {navigationOptions.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className={`navigation-option${navigationPosition === option.id ? " selected" : ""}`}
                  aria-pressed={navigationPosition === option.id}
                  onClick={() => onNavigationPositionChange(option.id)}
                >
                  <StudioIcon name={option.icon} size={17} />
                  <span>{t(option.labelKey)}</span>
                </button>
              ))}
            </div>
          </Card>
        </div>
      ) : null}

      {tab === "account" ? (
        <div
          className="settings-panel"
          id="settings-panel-account"
          role="tabpanel"
          aria-labelledby="settings-tab-account"
        >
          <Card tone="plain" className="settings-card">
            <h2>{t("settings-account-title")}</h2>
            <p className="settings-note">{t("settings-account-note")}</p>
            {learnerId ? <code className="settings-identity">{learnerId}</code> : null}
            <div className="method-list">
              {methodRow("email", "settings-method-email")}
              {methodRow("telegram", "settings-method-telegram")}
            </div>
          </Card>

          <div className="settings-two-column">
            <Card tone="plain" className="settings-card">
              <h2>{t("app-language")}</h2>
              <p className="settings-note">{t("settings-language-note")}</p>
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
              <h2>{t("settings-sync-title")}</h2>
              <p className="settings-note">{t("settings-sync-note")}</p>
              <div className="settings-sync-status">
                <span className={`sync-status-dot${status.online ? " online" : ""}`} />
                <strong>{status.online ? t("sync-online") : t("sync-offline")}</strong>
                <span>
                  {status.pendingOutbox > 0
                    ? t("settings-sync-pending", { count: status.pendingOutbox })
                    : t("settings-sync-clear")}
                </span>
              </div>
            </Card>
          </div>

          {onLogout ? (
            <Card tone="plain" className="settings-card settings-split-card">
              <div>
                <h2>{t("auth-sign-out")}</h2>
                <p className="settings-note">{t("profile-logout-note")}</p>
              </div>
              <Button type="button" variant="danger" size="sm" onClick={onLogout}>
                {t("auth-sign-out")}
              </Button>
            </Card>
          ) : null}
        </div>
      ) : null}

      {tab === "offline" ? (
        <div
          className="settings-panel"
          id="settings-panel-offline"
          role="tabpanel"
          aria-labelledby="settings-tab-offline"
        >
          <Card tone="plain" className="settings-card">
            <h2>{t("settings-offline-title")}</h2>
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

          <div className="settings-two-column">
            <Card tone="plain" className="settings-card">
              <h2>{t("settings-export-title")}</h2>
              <p className="settings-note">{t("settings-export-note")}</p>
              <Button type="button" variant="secondary" size="sm" onClick={() => void exportCache()}>
                {t("settings-export-action")}
              </Button>
            </Card>

            <Card tone="plain" className="settings-card">
              <h2>{t("settings-reset-title")}</h2>
              <p className="settings-note">{t("settings-reset-note")}</p>
              <div className="settings-options">
                <Button
                  type="button"
                  variant="danger"
                  size="sm"
                  onClick={() => {
                    if (!resetArmed) {
                      setResetArmed(true);
                      return;
                    }
                    setResetArmed(false);
                    void resetCache();
                  }}
                >
                  {resetArmed ? t("settings-reset-confirm") : t("settings-reset-action")}
                </Button>
                {resetArmed ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setResetArmed(false)}
                  >
                    {t("common-cancel")}
                  </Button>
                ) : null}
              </div>
            </Card>
          </div>

          {dataNotice ? <p className="storage-status storage-granted">{dataNotice}</p> : null}
          {dataError ? <p className="inline-alert danger" role="alert">{dataError}</p> : null}
        </div>
      ) : null}

      {tab === "admin" && isAdmin ? (
        <div
          className="settings-panel"
          id="settings-panel-admin"
          role="tabpanel"
          aria-labelledby="settings-tab-admin"
        >
          <Card tone="plain" className="settings-card">
            <h2>{t("settings-registration-title")}</h2>
            <p className="settings-note">{t("settings-registration-note")}</p>
          </Card>
          {adminApi ? <AdminAccessSection api={adminApi} /> : null}
        </div>
      ) : null}
    </div>
  );
}
