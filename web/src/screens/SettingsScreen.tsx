import { useEffect, useRef, useState } from "react";

import type { ApiClient, ProfileResponse } from "../api/client";
import {
  CrossfadeIcon,
  StudioIcon,
  type StudioIconName,
} from "../components/StudioIcons";
import { Button, Card, StatusMeta } from "../components/ui";
import type { OutboxRecord } from "../db/storage";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import { SOURCE_CODE_URL, SOURCE_LICENSE } from "../pwa/links";
import type { NavigationPosition } from "../prototype/layout";
import type { AppStatus } from "../prototype/status";
import {
  paletteForMode,
  themeMode,
  type PaletteName,
  type ThemeMode,
} from "../theme/tokens";
import { AdminAccessSection } from "./AdminAccessSection";

export type SettingsTab = "appearance" | "account" | "offline" | "admin";

export interface SettingsScreenProps {
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  theme: PaletteName;
  onThemeChange: (theme: PaletteName) => void;
  navigationPosition: NavigationPosition;
  onNavigationPositionChange: (position: NavigationPosition) => void;
  status: AppStatus;
  rejected: OutboxRecord[];
  minor: boolean;
  serverProfile: ProfileResponse | null;
  onExportCache: () => Promise<void>;
  onExportAccount: () => Promise<void>;
  onEraseAccount: () => Promise<void>;
  onResetCache: () => Promise<void>;
  isAdmin: boolean;
  adminApi: ApiClient | null;
  embedded?: boolean;
  authMethod?: "email" | "telegram";
  learnerId?: string;
  onLogout?: () => void;
  onSyncNow?: () => Promise<boolean>;
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
  navigationPosition,
  onNavigationPositionChange,
  status,
  rejected,
  minor,
  serverProfile,
  onExportCache,
  onExportAccount,
  onEraseAccount,
  onResetCache,
  isAdmin,
  adminApi,
  embedded = false,
  authMethod,
  learnerId,
  onLogout,
  onSyncNow,
}: SettingsScreenProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<SettingsTab>("account");
  const [resetArmed, setResetArmed] = useState(false);
  const [eraseArmed, setEraseArmed] = useState(false);
  const [dataNotice, setDataNotice] = useState<string | null>(null);
  const [dataError, setDataError] = useState<string | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);
  const [syncPulse, setSyncPulse] = useState(false);
  const syncPulseTimerRef = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (syncPulseTimerRef.current !== null) {
        window.clearTimeout(syncPulseTimerRef.current);
      }
    },
    [],
  );

  const tabs: SettingsTabItem[] = [
    { id: "account", labelKey: "settings-tab-account", icon: "profile" },
    { id: "offline", labelKey: "settings-tab-offline", icon: "offline" },
    { id: "appearance", labelKey: "settings-tab-appearance", icon: "palette" },
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

  const exportAccount = async () => {
    setDataNotice(null);
    setDataError(null);
    try {
      await onExportAccount();
      setDataNotice(t("settings-export-account-ready"));
    } catch {
      setDataError(t("settings-export-account-failed"));
    }
  };

  const eraseAccount = async () => {
    setDataNotice(null);
    setDataError(null);
    try {
      await onEraseAccount();
    } catch {
      setDataError(t("settings-erase-failed"));
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

  const syncNow = async () => {
    if (!onSyncNow || syncBusy) {
      return;
    }
    setSyncBusy(true);
    setSyncPulse(false);
    try {
      if (await onSyncNow()) {
        setSyncPulse(true);
        if (syncPulseTimerRef.current !== null) {
          window.clearTimeout(syncPulseTimerRef.current);
        }
        syncPulseTimerRef.current = window.setTimeout(() => {
          syncPulseTimerRef.current = null;
          setSyncPulse(false);
        }, 900);
      }
    } catch {
      // The sync status remains the source of truth on failure.
    } finally {
      setSyncBusy(false);
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
              <p>{t("profile-appearance-note")}</p>
            </div>
          </div>

          <div className="palette-options" role="group" aria-label={t("app-theme")}>
            {(["light", "dark"] as const satisfies readonly ThemeMode[]).map(
              (mode) => {
                const candidate = paletteForMode(mode);
                const selected = themeMode(theme) === mode;
                return (
                  <button
                    key={mode}
                    type="button"
                    className={`palette-option${selected ? " selected" : ""}`}
                    aria-pressed={selected}
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
                      <strong>{t(`theme-${mode}`)}</strong>
                      <span>{t(`theme-${mode}-note`)}</span>
                    </span>
                    <CrossfadeIcon
                      name="palette"
                      activeName="check"
                      active={selected}
                      size={16}
                      className="appearance-check"
                    />
                  </button>
                );
              },
            )}
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

          {minor ? (
            <Card tone="tinted" className="settings-card">
              <h2>{t("settings-child-title")}</h2>
              <p className="settings-note">{t("settings-child-note")}</p>
              <div className="settings-sync-status">
                <StudioIcon name="profile" size={16} />
                <strong>
                  {serverProfile?.privacy.profile_visibility === "private"
                    ? t("profile-visibility-self")
                    : t("profile-visibility-team")}
                </strong>
                <span>{t("settings-child-chat")}</span>
              </div>
            </Card>
          ) : null}

          <div className="settings-two-column">
            <Card tone="plain" className="settings-card">
              <h2>{t("settings-export-account-title")}</h2>
              <p className="settings-note">{t("settings-export-account-note")}</p>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                onClick={() => void exportAccount()}
              >
                {t("settings-export-account-action")}
              </Button>
            </Card>

            <Card tone="plain" className="settings-card">
              <h2>{t("settings-erase-title")}</h2>
              <p className="settings-note">{t("settings-erase-note")}</p>
              <div className="settings-options">
                <Button
                  type="button"
                  variant="danger"
                  size="sm"
                  onClick={() => {
                    if (!eraseArmed) {
                      setEraseArmed(true);
                      return;
                    }
                    setEraseArmed(false);
                    void eraseAccount();
                  }}
                >
                  {eraseArmed
                    ? t("settings-erase-confirm")
                    : t("settings-erase-action")}
                </Button>
                {eraseArmed ? (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setEraseArmed(false)}
                  >
                    {t("common-cancel")}
                  </Button>
                ) : null}
              </div>
            </Card>
          </div>

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
                <CrossfadeIcon
                  name="sync"
                  activeName="check"
                  active={status.online && status.pendingOutbox === 0}
                  size={16}
                  className="sync-status-icon"
                />
                <strong>{status.online ? t("sync-online") : t("sync-offline")}</strong>
                <span>
                  {status.pendingOutbox > 0
                    ? t("settings-sync-pending", { count: status.pendingOutbox })
                    : t("settings-sync-clear")}
                </span>
              </div>
              {onSyncNow ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="sync-action"
                  busy={syncBusy}
                  onClick={() => void syncNow()}
                >
                  <CrossfadeIcon
                    name="sync"
                    activeName="check"
                    active={syncPulse}
                    size={16}
                  />
                  {t("sync-retry")}
                </Button>
              ) : null}
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

          {dataNotice ? <p className="storage-status storage-granted">{dataNotice}</p> : null}
          {dataError ? <p className="inline-alert danger" role="alert">{dataError}</p> : null}

          <Card tone="plain" className="settings-card">
            <h2>{t("settings-source-title")}</h2>
            <p className="settings-note">{t("settings-source-note")}</p>
            <a
              className="settings-source-link"
              href={SOURCE_CODE_URL}
              target="_blank"
              rel="noreferrer"
            >
              {t("settings-source-action")} · {SOURCE_LICENSE}
            </a>
          </Card>
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
