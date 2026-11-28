import { useMemo, useState } from "react";

import type { ApiClient, ProfileResponse } from "../api/client";
import { IellaSprite } from "../components/IellaSprite";
import { StudioIcon } from "../components/StudioIcons";
import { Button, Card, ProgressBar, StatusMeta } from "../components/ui";
import type { OutboxRecord } from "../db/storage";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import type { PrototypeContent } from "../prototype/content";
import {
  kcProgress,
  levelView,
  memoryRetention,
  misconceptionProgress,
  parseLearnerState,
  statsSummary,
  totalReviews,
  weakestKcs,
} from "../prototype/progress";
import type { AppStatus } from "../prototype/status";
import type { PaletteName } from "../theme/tokens";
import { SettingsScreen } from "./SettingsScreen";

export type ProfileVisibility = "self" | "team" | "friends" | "everyone";

export interface ProfileScreenProps {
  learnerLabel: string;
  learnerId: string;
  authMethod: "email" | "telegram";
  content: PrototypeContent;
  stateJson: string | null;
  streak: number;
  isAdmin: boolean;
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
  adminApi: ApiClient | null;
  onLogout: () => void;
  onSyncNow?: () => Promise<boolean>;
}

const VISIBILITIES: ProfileVisibility[] = [
  "self",
  "team",
  "friends",
  "everyone",
];

interface Milestone {
  id: string;
  labelKey: string;
  noteKey: string;
  earned: boolean;
}

export function ProfileScreen({
  learnerLabel,
  learnerId,
  authMethod,
  content,
  stateJson,
  streak,
  isAdmin,
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
  adminApi,
  onLogout,
  onSyncNow,
}: ProfileScreenProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<"profile" | "settings">("profile");
  const [visibility, setVisibility] = useState<ProfileVisibility>("self");
  const [preview, setPreview] = useState(false);
  const state = useMemo(() => parseLearnerState(stateJson), [stateJson]);
  const progress = useMemo(
    () => kcProgress(content, state, locale),
    [content, state, locale],
  );
  const stats = statsSummary(progress);
  const reviews = totalReviews(progress);
  const retention = memoryRetention(progress);
  const due = progress.filter((kc) => kc.due).length;
  const level = levelView(progress, state.startLevel);
  const levelName = level.preA1 ? t("level-pre-a1") : level.current;
  const nextLevel = level.next;
  const levelPercent = level.percent;
  const levelMastered = level.mastered;
  const levelTotal = level.total;
  const errors = useMemo(
    () =>
      misconceptionProgress(content, state, locale)
        .filter((entry) => entry.manifested > 0 && entry.status !== "cured")
        .slice(0, 4),
    [content, state, locale],
  );
  const notes = weakestKcs(progress, 3);
  const visitor = preview;
  const privateVisitor = visitor && visibility === "self";
  const milestones: Milestone[] = [
    {
      id: "first-answer",
      labelKey: "profile-milestone-first-answer",
      noteKey: "profile-milestone-first-answer-note",
      earned: reviews > 0,
    },
    {
      id: "memory",
      labelKey: "profile-milestone-memory",
      noteKey: "profile-milestone-memory-note",
      earned: stats.inMemory > 0,
    },
    {
      id: "mastery",
      labelKey: "profile-milestone-mastery",
      noteKey: "profile-milestone-mastery-note",
      earned: stats.mastered > 0,
    },
    {
      id: "rhythm",
      labelKey: "profile-milestone-rhythm",
      noteKey: "profile-milestone-rhythm-note",
      earned: streak >= 3,
    },
    {
      id: "ten-reviews",
      labelKey: "profile-milestone-ten-reviews",
      noteKey: "profile-milestone-ten-reviews-note",
      earned: reviews >= 10,
    },
  ];

  return (
    <div className="profile-screen profile-personal">
      <header className="profile-heading">
        <div className="profile-heading-main">
          <div className="profile-avatar profile-avatar-large" aria-hidden="true">
            {learnerLabel.slice(0, 1)}
          </div>
          <div className="profile-heading-copy">
            <p className="eyebrow">{t("profile-eyebrow")}</p>
            <h1>{learnerLabel}</h1>
            {!visitor ? <code>{learnerId}</code> : null}
            <div className="profile-meta-line">
              <StatusMeta tone={isAdmin ? "warning" : "neutral"}>
                {isAdmin ? t("profile-role-admin") : t("profile-role-learner")}
              </StatusMeta>
              <span className="meta-separator" aria-hidden="true">
                ·
              </span>
              <StatusMeta tone="neutral">{t("profile-local")}</StatusMeta>
              {minor ? (
                <>
                  <span className="meta-separator" aria-hidden="true">
                    ·
                  </span>
                  <StatusMeta tone="accent" dot>
                    {t("profile-minor-badge")}
                  </StatusMeta>
                </>
              ) : null}
              {visitor ? (
                <>
                  <span className="meta-separator" aria-hidden="true">
                    ·
                  </span>
                  <StatusMeta tone="accent" dot>
                    {t("profile-preview-badge")}
                  </StatusMeta>
                </>
              ) : null}
            </div>
          </div>
        </div>
        <div className="profile-heading-actions">
          <span className="streak-meta">
            <StudioIcon name="streak" size={17} />
            <span>{t("today-streak", { days: streak })}</span>
          </span>
          <Button
            type="button"
            variant={preview ? "primary" : "secondary"}
            size="sm"
            aria-pressed={preview}
            onClick={() => setPreview((value) => !value)}
          >
            <StudioIcon name={preview ? "arrow-left" : "profile"} size={16} />
            {preview ? t("profile-preview-exit") : t("profile-preview-action")}
          </Button>
          <Button
            type="button"
            variant={view === "settings" ? "primary" : "secondary"}
            size="sm"
            aria-pressed={view === "settings"}
            onClick={() =>
              setView((current) =>
                current === "settings" ? "profile" : "settings",
              )
            }
          >
            <StudioIcon
              name={view === "settings" ? "arrow-left" : "gear"}
              size={16}
            />
            {view === "settings"
              ? t("profile-settings-back")
              : t("profile-settings-open")}
          </Button>
        </div>
      </header>

      {view === "settings" ? (
        <SettingsScreen
          embedded
          locale={locale}
          locales={locales}
          onLocaleChange={onLocaleChange}
          theme={theme}
          onThemeChange={onThemeChange}
          navigationPosition={navigationPosition}
          onNavigationPositionChange={onNavigationPositionChange}
          status={status}
          rejected={rejected}
          minor={minor}
          serverProfile={serverProfile}
          onExportCache={onExportCache}
          onExportAccount={onExportAccount}
          onEraseAccount={onEraseAccount}
          onResetCache={onResetCache}
          isAdmin={isAdmin}
          adminApi={adminApi}
          authMethod={authMethod}
          learnerId={learnerId}
          onLogout={onLogout}
          onSyncNow={onSyncNow}
        />
      ) : (
        <>
          {visitor ? (
            <div className="profile-preview-banner" role="status">
              <IellaSprite size={28} mood="curious" />
              <span>{t("profile-preview-banner")}</span>
              <StatusMeta tone="accent" dot>
                {t(`profile-visibility-`)}
              </StatusMeta>
            </div>
          ) : null}

          <div className="profile-public-grid">
            <Card tone="tinted" className="profile-banner-card">
              <div className="profile-banner-iella">
                <IellaSprite size={56} mood="proud" />
                <div>
                  <span>{t("iella-companion")}</span>
                  <strong>{t("profile-iella-badge", { level: levelName })}</strong>
                </div>
              </div>
              <div className="profile-level-block">
                <div className="profile-level-heading">
                  <span>{t("profile-level-progress")}</span>
                  <strong>
                    {levelName}
                    <StudioIcon name="arrow-right" size={13} />
                    {nextLevel}
                  </strong>
                </div>
                <ProgressBar
                  value={levelPercent}
                  tone="green"
                  label={t("profile-level-progress")}
                />
                <p>
                  {levelTotal === 0
                    ? level.preA1 && levelPercent > 0
                      ? t("level-start-note", {
                          percent: levelPercent,
                          next: nextLevel,
                        })
                      : t("today-level-empty")
                    : t("profile-level-note", {
                        mastered: levelMastered,
                        total: levelTotal,
                        level: levelName,
                      })}
                </p>
              </div>
            </Card>

            <Card tone="plain" className="profile-visibility-card">
              <div className="profile-section-heading">
                <div>
                  <h2>{t("profile-public-title")}</h2>
                  <p>{t("profile-public-note")}</p>
                </div>
                <StudioIcon name="globe" size={19} />
              </div>
              <div
                className="profile-visibility-options"
                role="group"
                aria-label={t("profile-visibility-title")}
              >
                {VISIBILITIES.map((candidate) => {
                  const allowed =
                    !minor || candidate === "self" || candidate === "team";
                  return (
                    <button
                      key={candidate}
                      type="button"
                      className={`profile-visibility-option${visibility === candidate ? " active" : ""}`}
                      aria-pressed={visibility === candidate}
                      disabled={!allowed}
                      onClick={() => setVisibility(candidate)}
                    >
                      {t(`profile-visibility-${candidate}`)}
                    </button>
                  );
                })}
              </div>
              {privateVisitor ? (
                <p className="profile-visibility-warning">
                  {t("profile-visibility-private")}
                </p>
              ) : null}
            </Card>
          </div>

          {privateVisitor ? (
            <Card tone="quiet" className="profile-private-card">
              <StudioIcon name="offline" size={20} />
              <div>
                <strong>{t("profile-visibility-private")}</strong>
                <p>{t("profile-public-note")}</p>
              </div>
            </Card>
          ) : (
            <>
              <div className="profile-overview">
                <Card tone="plain" className="profile-stat-card">
                  <span className="profile-stat-label">{t("stats-mastered")}</span>
                  <strong className="profile-stat-value">{stats.mastered}</strong>
                  <span className="profile-stat-note">
                    {t("profile-mastered-note", {
                      total: progress.length,
                      reviews,
                    })}
                  </span>
                  <ProgressBar
                    value={
                      progress.length === 0
                        ? 0
                        : Math.round((stats.mastered / progress.length) * 100)
                    }
                    tone="green"
                    label={t("stats-mastered")}
                  />
                </Card>

                <Card tone="plain" className="profile-stat-card">
                  <span className="profile-stat-label">{t("stats-memory")}</span>
                  <strong className="profile-stat-value">
                    {stats.inMemory}
                    <small>/{progress.length}</small>
                  </strong>
                  <span className="profile-stat-note">
                    {stats.inMemory === 0
                      ? t("stats-memory-empty")
                      : t("profile-memory-note", {
                          days: Math.max(1, Math.round(stats.averageStability)),
                          due,
                        })}
                  </span>
                  <ProgressBar
                    value={retention}
                    tone="blue"
                    label={t("stats-memory")}
                  />
                </Card>

                <Card tone="plain" className="profile-stat-card">
                  <span className="profile-stat-label">{t("profile-reviews")}</span>
                  <strong className="profile-stat-value">{reviews}</strong>
                  <span className="profile-stat-note">{t("profile-reviews-note")}</span>
                  <div className="profile-review-meta">
                    <span>
                      <strong>{streak}</strong> {t("profile-streak-short")}
                    </span>
                    <span className="meta-separator" aria-hidden="true">
                      ·
                    </span>
                    <span>
                      <strong>{due}</strong> {t("today-memory-due-label")}
                    </span>
                  </div>
                </Card>
              </div>

              <section className="profile-section" aria-labelledby="profile-milestones-title">
                <div className="profile-section-heading">
                  <div>
                    <h2 id="profile-milestones-title">{t("profile-milestones")}</h2>
                    <p>{t("profile-milestones-note")}</p>
                  </div>
                </div>
                <div className="milestone-grid">
                  {milestones.map((milestone) => (
                    <Card
                      key={milestone.id}
                      tone={milestone.earned ? "plain" : "quiet"}
                      className={`milestone-badge${milestone.earned ? " earned" : ""}`}
                    >
                      <span className="milestone-icon" aria-hidden="true">
                        <StudioIcon
                          name={milestone.earned ? "check" : "spark"}
                          size={18}
                        />
                      </span>
                      <strong>{t(milestone.labelKey)}</strong>
                      <span>{t(milestone.noteKey)}</span>
                    </Card>
                  ))}
                </div>
              </section>

              {!visitor ? (
                <>
                  <section
                    className="profile-section"
                    aria-labelledby="profile-errors-title"
                  >
                    <div className="profile-section-heading">
                      <div>
                        <h2 id="profile-errors-title">{t("profile-error-notebook")}</h2>
                        <p>{t("profile-error-note")}</p>
                      </div>
                      <StatusMeta tone={errors.length > 0 ? "danger" : "neutral"} dot>
                        {errors.length}
                      </StatusMeta>
                    </div>
                    {errors.length === 0 ? (
                      <p className="chart-empty">{t("profile-error-empty")}</p>
                    ) : (
                      <div className="profile-error-list">
                        {errors.map((error) => (
                          <div className="profile-error-row" key={error.id}>
                            <span className="profile-error-mark" aria-hidden="true">
                              ×
                            </span>
                            <div>
                              <strong>{error.title}</strong>
                              <span>{t(`progress-status-${error.status}`)}</span>
                            </div>
                            <StatusMeta tone="danger">
                              {t("profile-error-count", { count: error.manifested })}
                            </StatusMeta>
                          </div>
                        ))}
                      </div>
                    )}
                  </section>

                  <section
                    className="profile-section"
                    aria-labelledby="profile-notes-title"
                  >
                    <div className="profile-section-heading">
                      <div>
                        <h2 id="profile-notes-title">{t("profile-notes-title")}</h2>
                        <p>{t("profile-notes-note")}</p>
                      </div>
                    </div>
                    {notes.length === 0 ? (
                      <p className="chart-empty">{t("profile-notes-empty")}</p>
                    ) : (
                      <div className="profile-note-list">
                        {notes.map((note) => (
                          <div className="profile-note-row" key={note.kcId}>
                            <div>
                              <strong>{note.title}</strong>
                              <span>{t("profile-note-progress", { percent: note.percent })}</span>
                            </div>
                            <ProgressBar
                              value={note.percent}
                              tone="blue"
                              label={note.title}
                            />
                          </div>
                        ))}
                      </div>
                    )}
                    <p className="profile-private-note">{t("profile-notes-private")}</p>
                  </section>
                </>
              ) : null}
            </>
          )}
        </>
      )}
    </div>
  );
}
