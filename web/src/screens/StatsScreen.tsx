import { useMemo } from "react";

import { Badge, Card, Disclosure, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import type { PrototypeContent } from "../prototype/content";
import { kcProgress, parseLearnerState, statsSummary } from "../prototype/progress";

export interface StatsScreenProps {
  content: PrototypeContent;
  stateJson: string | null;
}

export function StatsScreen({ content, stateJson }: StatsScreenProps) {
  const { t, locale } = useTranslation();
  const progress = useMemo(
    () => kcProgress(content, parseLearnerState(stateJson), locale),
    [content, stateJson, locale],
  );
  const stats = statsSummary(progress);
  const masteredPercent =
    progress.length === 0
      ? 0
      : Math.round((stats.mastered / progress.length) * 100);

  return (
    <div className="stats-screen">
      <header className="page-heading">
        <div>
          <p className="eyebrow">{t("stats-eyebrow")}</p>
          <h1>{t("stats-title")}</h1>
        </div>
        <Badge tone="blue">{t("stats-articles")}</Badge>
      </header>

      <div className="stats-grid">
        <Card tone="plain" className="stat-card">
          <div className="stat-head">
            <p className="stat-label">{t("stats-mastered")}</p>
            <span className="stat-value">{stats.mastered}</span>
          </div>
          <ProgressBar value={masteredPercent} tone="green" />
          <p className="stat-note">
            {t("stats-mastered-note", {
              total: progress.length,
            })}
          </p>
          <Disclosure summary={t("stats-where")}>
            <p>{t("stats-where-mastered")}</p>
          </Disclosure>
        </Card>

        <Card tone="plain" className="stat-card">
          <div className="stat-head">
            <p className="stat-label">{t("stats-memory")}</p>
            <span className="stat-value">{stats.inMemory}</span>
          </div>
          <ProgressBar
            value={Math.min(100, (stats.averageStability / 30) * 100)}
            tone="purp"
          />
          <p className="stat-note">
            {stats.averageStability > 0
              ? t("stats-memory-note", {
                  days: Math.max(1, Math.round(stats.averageStability)),
                })
              : t("stats-memory-empty")}
          </p>
          <Disclosure summary={t("stats-where")}>
            <p>{t("stats-where-memory")}</p>
          </Disclosure>
        </Card>

        <Card tone="plain" className="stat-card">
          <div className="stat-head">
            <p className="stat-label">{t("stats-calibration")}</p>
            <span className="stat-value">
              {stats.calibration === null ? "—" : `${stats.calibration}%`}
            </span>
          </div>
          <p className="stat-note">
            {stats.calibration === null
              ? t("stats-calibration-empty")
              : t("stats-calibration-note")}
          </p>
          <Disclosure summary={t("stats-where")}>
            <p>{t("stats-where-calibration")}</p>
          </Disclosure>
        </Card>
      </div>

      <Card tone="tinted" className="kc-table">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("stats-kc-eyebrow")}</p>
            <h2>{t("stats-kc-title")}</h2>
          </div>
        </div>
        <div className="kc-table-list">
          {progress.map((kc) => (
            <div className="kc-row" key={kc.kcId}>
              <div className="kc-row-title">
                <strong>{kc.title}</strong>
                <span>
                  {kc.mastered
                    ? t("today-kc-mastered")
                    : t("today-kc-progress")}
                </span>
              </div>
              <div className="kc-row-progress">
                <ProgressBar
                  value={kc.percent}
                  tone={kc.mastered ? "green" : "blue"}
                />
                <span>{kc.percent}%</span>
              </div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
