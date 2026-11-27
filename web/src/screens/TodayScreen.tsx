import { useMemo, useState } from "react";

import { HeroIllustration } from "../components/HeroIllustration";
import { IellaTour } from "../components/IellaTour";
import { StudioIcon } from "../components/StudioIcons";
import { IellaSprite } from "../components/IellaSprite";
import { Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import type { PrototypeContent } from "../prototype/content";
import {
  kcProgress,
  parseLearnerState,
} from "../prototype/progress";
import type { AppStatus } from "../prototype/status";

export interface TodayScreenProps {
  content: PrototypeContent;
  stateJson: string | null;
  learnerLabel: string;
  streak: number;
  status: AppStatus;
  onStart: () => void;
  onOpenStats: () => void;
  tourOpen: boolean;
  onTourComplete: () => void;
}

const IELLA_TIPS = [
  "iella-tip-route",
  "iella-tip-offline",
  "iella-tip-memory",
] as const;

export function TodayScreen({
  content,
  stateJson,
  learnerLabel,
  streak,
  status,
  onStart,
  onOpenStats,
  tourOpen,
  onTourComplete,
}: TodayScreenProps) {
  const { t, locale } = useTranslation();
  const [tipIndex, setTipIndex] = useState<number | null>(null);
  const progress = useMemo(
    () => kcProgress(content, parseLearnerState(stateJson), locale),
    [content, stateJson, locale],
  );
  const currentLevel = content.visibleArticles(locale).reduce(
    (level, entry) =>
      entry.level_rank > level.rank
        ? { label: entry.kc.level, rank: entry.level_rank }
        : level,
    { label: "A1", rank: 0 },
  ).label;

  const statusCues: Array<{
    label: string;
    tone: "warning" | "danger";
    icon: "sync" | "offline";
  }> = [];
  if (!status.online) {
    statusCues.push({
      label: t("sync-offline"),
      tone: "warning",
      icon: "offline",
    });
  }
  if (status.pendingOutbox > 0) {
    statusCues.push({
      label: t("sync-pending", { count: status.pendingOutbox }),
      tone: "warning",
      icon: "sync",
    });
  }
  if (status.rejectedOutbox > 0) {
    statusCues.push({
      label: t("sync-rejected", { count: status.rejectedOutbox }),
      tone: "danger",
      icon: "offline",
    });
  }
  if (status.clockDrift) {
    statusCues.push({
      label: t("sync-clock-warning"),
      tone: "warning",
      icon: "sync",
    });
  }

  const tip = tipIndex === null ? null : t(IELLA_TIPS[tipIndex] ?? IELLA_TIPS[0]);

  return (
    <div className="today-screen">
      <header className="today-masthead">
        <div className="today-identity">
          <span className="today-avatar" aria-hidden="true">
            {learnerLabel.slice(0, 1)}
          </span>
          <h1>{t("today-greeting", { name: learnerLabel })}</h1>
        </div>
        <div className="today-masthead-side">
          <span className="streak-badge">
            <StudioIcon name="streak" size={17} />
            <span>{t("today-streak", { days: streak })}</span>
          </span>
          {statusCues.length > 0 ? (
            <div className="today-status" role="status" aria-live="polite">
              {statusCues.map((cue) => (
                <span
                  key={cue.label}
                  className={`sync-cue sync-cue-${cue.tone}`}
                >
                  <StudioIcon name={cue.icon} size={14} />
                  {cue.label}
                </span>
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

      <section className="today-route" data-tour="route" aria-labelledby="today-route-title">
        <HeroIllustration />
        <span
          className="iella-anchor iella-anchor-route"
          data-iella-perch="today-route"
          data-iella-route="today"
          aria-hidden="true"
        />
        <div className="route-topline">
          <span className="route-chip route-chip-time">
            {t("session-budget", { minutes: 25 })}
          </span>
        </div>
        <div className="route-copy">
          <h2 id="today-route-title">{t("today-route-title")}</h2>
          <p>{t("today-route-body")}</p>
          <div className="today-hero-actions">
            <Button
              type="button"
              variant="primary"
              size="lg"
              data-tour="start"
              onClick={onStart}
            >
              {t("session-start")}
            </Button>
          </div>
        </div>
        <div className="route-level" data-tour="level">
          <span>{t("today-level")}</span>
          <strong>{currentLevel}</strong>
        </div>
      </section>

      <section className="articles-block" aria-labelledby="today-articles-title">
        <div className="articles-heading">
          <div>
            <h2 id="today-articles-title">{t("today-articles-title")}</h2>
          </div>
          <div className="articles-heading-side">
            <span className="articles-count">
              {t("today-articles-count", { count: progress.length })}
            </span>
            <Button type="button" variant="ghost" size="sm" onClick={onOpenStats}>
              {t("today-open-stats")}
            </Button>
          </div>
        </div>
        <div className="kc-grid">
          {progress.map((kc, index) => (
            <Card
              key={kc.kcId}
              tone="plain"
              className={`kc-card${kc.mastered ? " kc-card-mastered" : ""}`}
            >
              {index === 0 ? (
                <span
                  className="iella-anchor iella-anchor-card"
                  data-iella-perch="today-card"
                  data-iella-route="today"
                  aria-hidden="true"
                />
              ) : null}
              <div className="kc-card-topline">
                <span className="kc-index" aria-hidden="true">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <span className="kc-meta">
                  {kc.mastered
                    ? t("today-kc-mastered")
                    : kc.observations > 0
                      ? t("today-kc-progress")
                      : t("today-kc-new")}
                </span>
              </div>
              <div className="kc-card-body">
                <h3>{kc.title}</h3>
                <span className="kc-percent" aria-hidden="true">
                  {kc.percent}
                  <span>%</span>
                </span>
              </div>
              <ProgressBar
                value={kc.percent}
                tone={kc.mastered ? "green" : kc.observations > 0 ? "coral" : "blue"}
                label={kc.title}
              />
              <p className="kc-note">
                {kc.stabilityDays > 0
                  ? t("today-kc-stability", {
                      days: Math.max(1, Math.round(kc.stabilityDays)),
                    })
                  : t("today-kc-no-memory")}
              </p>
            </Card>
          ))}
        </div>
      </section>

      <Card tone="tinted" className="iella-note">
        <IellaSprite
          size={58}
          mood="curious"
          label={t("iella-tip-open")}
          onActivate={() =>
            setTipIndex((current) =>
              current === null ? 0 : (current + 1) % IELLA_TIPS.length,
            )
          }
        />
        <div>
          <strong>{t("iella-note-title")}</strong>
          <p aria-live="polite">{tip ?? t("iella-note-body")}</p>
        </div>
      </Card>

      <IellaTour open={tourOpen} onComplete={onTourComplete} />
    </div>
  );
}
