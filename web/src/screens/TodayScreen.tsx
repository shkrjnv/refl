import { useMemo } from "react";

import { HeroIllustration } from "../components/HeroIllustration";
import { IellaTour } from "../components/IellaTour";
import { IellaSprite } from "../components/IellaSprite";
import { Badge, Button, Card, ProgressBar } from "../components/ui";
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
  status: AppStatus;
  onStart: () => void;
  onOpenStats: () => void;
  tourOpen: boolean;
  onTourComplete: () => void;
}

export function TodayScreen({
  content,
  stateJson,
  learnerLabel,
  status,
  onStart,
  onOpenStats,
  tourOpen,
  onTourComplete,
}: TodayScreenProps) {
  const { t, locale } = useTranslation();
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

  const syncLabel = status.syncing
    ? t("sync-syncing")
    : !status.online
      ? t("sync-offline")
      : status.pendingOutbox === 0 && status.lastSyncedAt !== null
        ? t("sync-synced")
        : t("sync-online");

  return (
    <div className="today-screen">
      <header className="page-heading">
        <div>
          <p className="eyebrow">{t("today-eyebrow")}</p>
          <h1>{t("today-greeting", { name: learnerLabel })}</h1>
        </div>
        <Badge tone={status.online ? "green" : "sun"}>{syncLabel}</Badge>
      </header>

      <section className="status-strip" aria-label={t("sync-status-label")}>
        <span className={`status-pill ${status.online ? "online" : "offline"}`}>
          <span className="status-dot" aria-hidden="true" />
          {syncLabel}
        </span>
        {status.pendingOutbox > 0 ? (
          <span className="status-note">
            {t("sync-pending", { count: status.pendingOutbox })}
          </span>
        ) : null}
        {status.rejectedOutbox > 0 ? (
          <span className="status-note danger">
            {t("sync-rejected", { count: status.rejectedOutbox })}
          </span>
        ) : null}
        {status.clockDrift ? (
          <span className="status-note warning">{t("sync-clock-warning")}</span>
        ) : null}
        {status.persistence === "denied" ? (
          <span className="status-note warning">
            {t("storage-persist-denied")}
          </span>
        ) : null}
        {status.persistence === "unsupported" ? (
          <span className="status-note warning">
            {t("storage-unsupported")}
          </span>
        ) : null}
        {status.persistence === "granted" ? (
          <span className="status-note success">
            {t("storage-persist-granted")}
          </span>
        ) : null}
      </section>

      {status.updateReady ? (
        <div className="update-banner" role="status">
          <span>{t("sync-update-ready")}</span>
          <span>{t("sync-update-deferred")}</span>
        </div>
      ) : null}

      <section className="today-hero" data-tour="route">
        <HeroIllustration />
        <div className="today-hero-copy">
          <p className="eyebrow">{t("today-route-eyebrow")}</p>
          <h2>{t("today-route-title")}</h2>
          <p>{t("today-route-body")}</p>
          <div className="today-hero-actions">
            <Button type="button" variant="primary" size="lg" data-tour="start" onClick={onStart}>
              {t("session-start")}
            </Button>
            <span className="route-time">
              {t("session-budget", { minutes: 25 })}
            </span>
          </div>
        </div>
        <span className="level-stamp" data-tour="level">
          <span>{t("today-level")}</span>
          <strong>{currentLevel}</strong>
        </span>
      </section>

      <section className="section-block">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("today-articles-eyebrow")}</p>
            <h2>{t("today-articles-title")}</h2>
          </div>
          <Button type="button" variant="ghost" size="sm" onClick={onOpenStats}>
            {t("today-open-stats")}
          </Button>
        </div>
        <div className="kc-grid">
          {progress.map((kc) => (
            <Card key={kc.kcId} tone="plain" className="kc-card">
              <div className="kc-card-head">
                <div>
                  <h3>{kc.title}</h3>
                  <span className="kc-meta">
                    {kc.mastered
                      ? t("today-kc-mastered")
                      : kc.observations > 0
                        ? t("today-kc-progress")
                        : t("today-kc-new")}
                  </span>
                </div>
                <Badge tone={kc.mastered ? "green" : "blue"}>
                  {kc.percent}%
                </Badge>
              </div>
              <ProgressBar
                value={kc.percent}
                tone={kc.mastered ? "green" : "blue"}
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
        <IellaSprite size={56} />
        <div>
          <strong>{t("iella-note-title")}</strong>
          <p>{t("iella-note-body")}</p>
        </div>
      </Card>

      <IellaTour open={tourOpen} onComplete={onTourComplete} />
    </div>
  );
}
