import { useMemo, useState } from "react";

import { HeroIllustration } from "../components/HeroIllustration";
import { IellaTip } from "../components/IellaTip";
import { IellaTour } from "../components/IellaTour";
import { CrossfadeIcon, StudioIcon } from "../components/StudioIcons";
import { IellaSprite } from "../components/IellaSprite";
import { Button, Card, ProgressBar, StatusMeta } from "../components/ui";
import type { StatusTone } from "../components/ui";
import { useTranslation } from "../i18n";
import type { ActivitySummary } from "../prototype/activity";
import type { PrototypeContent } from "../prototype/content";
import {
  dueCount,
  kcProgress,
  levelView,
  misconceptionProgress,
  parseLearnerState,
  totalReviews,
  weakestKcs,
  type KcProgress,
  type MisconceptionProgress,
} from "../prototype/progress";
import type { AppStatus } from "../prototype/status";

export interface TodayScreenProps {
  content: PrototypeContent;
  stateJson: string | null;
  learnerLabel: string;
  streak: number;
  status: AppStatus;
  activity: ActivitySummary;
  onStart: (minutes: number) => void;
  onOpenStats: () => void;
  tourOpen: boolean;
  onTourComplete: () => void;
}

const DURATIONS = [10, 15, 25] as const;

interface MasteryStatus {
  key: string;
  tone: "mastered" | "fading" | "learning" | "not-started";
}

function masteryStatus(kc: KcProgress): MasteryStatus {
  if (kc.mastered) {
    return { key: "today-status-mastered", tone: "mastered" };
  }
  if (kc.due) {
    return { key: "today-status-fading", tone: "fading" };
  }
  if (kc.observations > 0) {
    return { key: "today-status-learning", tone: "learning" };
  }
  return { key: "today-status-not-started", tone: "not-started" };
}

function masteryTone(tone: MasteryStatus["tone"]): StatusTone {
  switch (tone) {
    case "mastered":
      return "success";
    case "fading":
      return "danger";
    case "learning":
      return "accent";
    default:
      return "neutral";
  }
}

function activityLevel(count: number): number {
  if (count <= 0) {
    return 0;
  }
  if (count < 3) {
    return 1;
  }
  if (count < 6) {
    return 2;
  }
  if (count < 10) {
    return 3;
  }
  return 4;
}

interface NoteItem {
  key: string;
  labelKey: string;
  title: string;
  detail: string;
  tone: "focus" | "watch";
}

export function TodayScreen({
  content,
  stateJson,
  learnerLabel,
  streak,
  status,
  activity,
  onStart,
  onOpenStats,
  tourOpen,
  onTourComplete,
}: TodayScreenProps) {
  const { t, locale } = useTranslation();
  const [duration, setDuration] = useState<(typeof DURATIONS)[number]>(25);
  const [openRule, setOpenRule] = useState<string | null>(null);
  const state = useMemo(() => parseLearnerState(stateJson), [stateJson]);
  const progress = useMemo(
    () => kcProgress(content, state, locale),
    [content, state, locale],
  );
  const due = dueCount(progress);
  const reviews = totalReviews(progress);
  const weak = weakestKcs(progress);
  const level = levelView(progress, state.startLevel);
  const levelName = level.preA1 ? t("level-pre-a1") : level.current;
  const nextLevel = level.next;
  const levelPercent = level.percent;
  const levelMastered = level.mastered;
  const levelTotal = level.total;
  const overallPercent =
    progress.length === 0
      ? 0
      : Math.round(
          progress.reduce((sum, kc) => sum + kc.percent, 0) / progress.length,
        );
  const errors = useMemo(
    () =>
      misconceptionProgress(content, state, locale)
        .filter((entry) => entry.manifested > 0 && entry.status !== "cured")
        .slice(0, 4),
    [content, state, locale],
  );
  const notes = useMemo<NoteItem[]>(() => {
    const items: NoteItem[] = weak.slice(0, 2).map((kc) => ({
      key: `focus-${kc.kcId}`,
      labelKey: "today-notes-review",
      title: kc.title,
      detail: t("today-notes-review-note", { percent: kc.percent }),
      tone: "focus",
    }));
    for (const error of errors.filter((entry) => entry.status === "active")) {
      items.push({
        key: `watch-${error.id}`,
        labelKey: "today-notes-watch",
        title: error.title,
        detail: t("today-notes-watch-note", {
          percent: Math.round(error.strength * 100),
        }),
        tone: "watch",
      });
      if (items.length >= 3) {
        break;
      }
    }
    return items.slice(0, 3);
  }, [errors, weak, t]);

  const statusCues: Array<{
    label: string;
    tone: "warning" | "danger";
  }> = [];
  if (!status.online) {
    statusCues.push({
      label: t("sync-offline"),
      tone: "warning",
    });
  }
  if (status.pendingOutbox > 0) {
    statusCues.push({
      label: t("sync-pending", { count: status.pendingOutbox }),
      tone: "warning",
    });
  }
  if (status.rejectedOutbox > 0) {
    statusCues.push({
      label: t("sync-rejected", { count: status.rejectedOutbox }),
      tone: "danger",
    });
  }
  if (status.clockDrift) {
    statusCues.push({
      label: t("sync-clock-warning"),
      tone: "warning",
    });
  }

  const renderErrorRow = (error: MisconceptionProgress) => (
    <div className="error-notebook-row" key={error.id}>
      <span className="error-notebook-mark" aria-hidden="true">
        ×
      </span>
      <span className="error-notebook-copy">
        <strong>{error.title}</strong>
        <span>{t(`progress-status-${error.status}`)}</span>
      </span>
      <span className="error-notebook-count">
        {t("today-error-count", { count: error.manifested })}
      </span>
    </div>
  );

  return (
    <div className="today-screen">
      <header className="today-masthead">
        <div className="today-identity">
          <span className="today-avatar" aria-hidden="true">
            {learnerLabel.slice(0, 1)}
          </span>
          <div>
            <p className="eyebrow">{t("today-eyebrow")}</p>
            <h1>{t("today-greeting", { name: learnerLabel })}</h1>
          </div>
        </div>
        <div className="today-masthead-side">
          <span className="streak-meta">
            <StudioIcon name="streak" size={17} />
            <span>{t("today-streak", { days: streak })}</span>
          </span>
          {statusCues.length > 0 ? (
            <div className="today-status" role="status" aria-live="polite">
              {statusCues.map((cue) => (
                <StatusMeta
                  key={cue.label}
                  tone={cue.tone === "danger" ? "danger" : "warning"}
                  dot
                >
                  {cue.label}
                </StatusMeta>
              ))}
            </div>
          ) : null}
        </div>
      </header>

      {status.updateReady ? (
        <div className="update-banner" role="status">
          <span>{t("sync-update-ready")}</span>
          <span>{t("sync-update-deferred")}</span>
        </div>
      ) : null}

      <div className="today-bento">
        <section
          className="today-hero-banner"
          data-tour="route"
          aria-labelledby="today-route-title"
        >
          <HeroIllustration />
          <div className="today-hero-scrim" aria-hidden="true" />
          <div className="today-hero-content">
            <div className="today-hero-topline">
              <span className="today-hero-companion">
                <IellaSprite
                  size={44}
                  mood="perched"
                  className="today-hero-iella"
                />
                <span>{t("iella-name")}</span>
                <span className="meta-separator" aria-hidden="true">
                  ·
                </span>
                <span>{t("iella-companion")}</span>
              </span>
            </div>
            <div className="today-hero-copy">
              <span className="bento-kicker">{t("today-route-eyebrow")}</span>
              <h2 id="today-route-title">{t("today-route-title")}</h2>
              <p>{t("today-route-body")}</p>
            </div>
            <div className="today-hero-actions">
              <div
                className="duration-pills"
                role="group"
                aria-label={t("today-duration-label")}
              >
                {DURATIONS.map((minutes) => (
                  <button
                    key={minutes}
                    type="button"
                    className={`duration-pill${duration === minutes ? " active" : ""}`}
                    aria-pressed={duration === minutes}
                    onClick={() => setDuration(minutes)}
                  >
                    {t("today-duration", { minutes })}
                  </button>
                ))}
              </div>
              <Button
                type="button"
                variant="primary"
                size="lg"
                className="route-start"
                data-tour="start"
                onClick={() => onStart(duration)}
              >
                {t("session-start")}
                <StudioIcon name="arrow-right" size={17} />
              </Button>
              <StatusMeta tone="neutral" dot className="today-hero-level">
                {levelName} → {nextLevel}
              </StatusMeta>
            </div>
          </div>
        </section>

        <Card
          tone="plain"
          className="bento-card bento-level"
          data-tour="level"
          aria-labelledby="today-level-title"
        >
          <div className="bento-card-heading">
            <span className="bento-kicker" id="today-level-title">
              {t("today-level-progress")}
            </span>
            <StatusMeta tone="accent" dot>
              {levelName} → {nextLevel}
            </StatusMeta>
          </div>
          <div className="level-figure">
            <strong>{levelPercent}%</strong>
          </div>
          <ProgressBar
            value={levelPercent}
            tone="green"
            label={t("today-level-progress")}
          />
          <p className="bento-note">
            {levelTotal === 0
              ? level.preA1 && levelPercent > 0
                ? t("level-start-note", {
                    percent: levelPercent,
                    next: nextLevel,
                  })
                : t("today-level-empty")
              : t("today-level-note", {
                  mastered: levelMastered,
                  total: levelTotal,
                })}
          </p>
        </Card>

        <Card
          tone="plain"
          className="bento-card bento-due"
          aria-labelledby="today-due-title"
        >
          <div className="bento-card-heading">
            <span className="bento-kicker">{t("today-due-kicker")}</span>
            <StudioIcon name="sync" size={17} />
          </div>
          <h2 id="today-due-title">{t("today-due-title")}</h2>
          <div className="due-metric">
            <strong className="due-value">{due}</strong>
            <span>{t("today-memory-due-label")}</span>
          </div>
          <p className="bento-note">
            {due === 0
              ? t("today-due-empty")
              : t("today-due-observations", {
                  count: reviews,
                  rules: progress.length,
                })}
          </p>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={due === 0}
            onClick={() => onStart(duration)}
          >
            {t("today-review-due")}
          </Button>
        </Card>

        <Card
          tone="plain"
          className="bento-card bento-activity"
          aria-labelledby="today-activity-title"
        >
          <div className="bento-card-heading">
            <div>
              <span className="bento-kicker">{t("today-activity-kicker")}</span>
              <h2 id="today-activity-title">{t("today-activity-title")}</h2>
            </div>
            <span className="activity-best">
              {t("today-activity-best", { days: activity.bestStreak })}
            </span>
          </div>
          <div className="activity-calendar">
            <div className="activity-weekday-labels" aria-hidden="true">
              <span />
              <span>{t("today-weekday-mon")}</span>
              <span />
              <span>{t("today-weekday-wed")}</span>
              <span />
              <span>{t("today-weekday-fri")}</span>
              <span />
            </div>
            <div
              className="activity-heatmap"
              role="img"
              aria-label={t("today-activity-summary", {
                total: activity.total,
                best: activity.bestStreak,
              })}
            >
              {activity.days.map((day) => (
                <span
                  key={day.day}
                  className={`activity-cell activity-level-${activityLevel(day.count)}`}
                  title={t("today-activity-cell", {
                    date: new Intl.DateTimeFormat(locale, {
                      month: "short",
                      day: "numeric",
                    }).format(new Date(`${day.day}T12:00:00`)),
                    count: day.count,
                  })}
                  aria-hidden="true"
                />
              ))}
            </div>
          </div>
          <div className="activity-footer">
            <p className="activity-note">
              {activity.total === 0
                ? t("today-activity-empty")
                : t("today-activity-today", { count: activity.todayCount })}
            </p>
            <div
              className="activity-legend"
              aria-label={t("today-activity-legend")}
            >
              <span>{t("today-activity-less")}</span>
              <i className="activity-cell activity-level-0" />
              <i className="activity-cell activity-level-1" />
              <i className="activity-cell activity-level-2" />
              <i className="activity-cell activity-level-3" />
              <i className="activity-cell activity-level-4" />
              <span>{t("today-activity-more")}</span>
            </div>
          </div>
        </Card>

        <Card
          tone="plain"
          className="bento-card bento-mastery"
          aria-labelledby="today-articles-title"
        >
          <div className="bento-card-heading">
            <div>
              <span className="bento-kicker">{t("today-articles-eyebrow")}</span>
              <h2 id="today-articles-title">{t("today-articles-title")}</h2>
            </div>
            <span className="articles-count">
              {t("today-articles-count", { count: progress.length })} ·{" "}
              {overallPercent}%
            </span>
          </div>
          <div className="mastery-list">
            {progress.slice(0, 5).map((kc) => {
              const status = masteryStatus(kc);
              return (
                <div className="mastery-row" key={kc.kcId}>
                  <button
                    type="button"
                    className="mastery-row-heading"
                    aria-expanded={openRule === kc.kcId}
                    onClick={() =>
                      setOpenRule((current) =>
                        current === kc.kcId ? null : kc.kcId,
                      )
                    }
                  >
                    <strong>{kc.title}</strong>
                    <span className="mastery-row-state">
                      <StatusMeta tone={masteryTone(status.tone)} dot>
                        {t(status.key)}
                      </StatusMeta>
                      <CrossfadeIcon
                        name="chevron-down"
                        activeName="chevron-up"
                        active={openRule === kc.kcId}
                        size={14}
                      />
                    </span>
                  </button>
                  <div className="mastery-row-bar">
                    <ProgressBar
                      value={kc.percent}
                      tone={kc.mastered ? "green" : kc.due ? "coral" : "blue"}
                      label={kc.title}
                    />
                    <span>{kc.percent}%</span>
                  </div>
                  <div
                    className={`mastery-detail${openRule === kc.kcId ? " open" : ""}`}
                    aria-hidden={openRule !== kc.kcId}
                  >
                    <div className="mastery-detail-inner">
                      {kc.stabilityDays > 0
                        ? t("today-kc-stability", {
                            days: Math.max(1, Math.round(kc.stabilityDays)),
                          })
                        : t("today-kc-no-memory")}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mastery-open"
            onClick={onOpenStats}
          >
            {t("today-open-stats")}
          </Button>
        </Card>

        <Card
          tone="plain"
          className="bento-card bento-errors"
          aria-labelledby="today-error-title"
        >
          <div className="bento-card-heading">
            <div>
              <span className="bento-kicker">{t("today-error-kicker")}</span>
              <h2 id="today-error-title">{t("today-error-notebook")}</h2>
            </div>
          </div>
          {errors.length === 0 ? (
            <p className="bento-empty">{t("today-error-empty")}</p>
          ) : (
            <div className="error-notebook-list">{errors.map(renderErrorRow)}</div>
          )}
          <p className="bento-note">{t("today-error-note")}</p>
        </Card>

        <Card
          tone="plain"
          className="bento-card bento-notes"
          aria-labelledby="today-notes-title"
        >
          <div className="bento-card-heading">
            <div>
              <span className="bento-kicker">{t("today-notes-kicker")}</span>
              <h2 id="today-notes-title">{t("today-notes-title")}</h2>
            </div>
            <StudioIcon name="practice" size={18} />
          </div>
          {notes.length === 0 ? (
            <p className="bento-empty">{t("today-notes-empty")}</p>
          ) : (
            <div className="notes-list">
              {notes.map((note) => (
                <div className={`note-row note-row-${note.tone}`} key={note.key}>
                  <span className="note-row-label">{t(note.labelKey)}</span>
                  <strong>{note.title}</strong>
                  <span>{note.detail}</span>
                </div>
              ))}
            </div>
          )}
          <p className="bento-note">{t("today-notes-private")}</p>
        </Card>
      </div>

      {!tourOpen ? <IellaTip section="notes" /> : null}
      <IellaTour open={tourOpen} onComplete={onTourComplete} />
    </div>
  );
}
