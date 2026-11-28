import { IellaSprite } from "../components/IellaSprite";
import { Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import type { LessonSummary } from "../prototype/lesson";

export interface SummaryScreenProps {
  summary: LessonSummary;
  onDone: () => void;
}

export function SummaryScreen({ summary, onDone }: SummaryScreenProps) {
  const { t } = useTranslation();
  const minutes = Math.max(1, Math.round(summary.durationMs / 60000));
  return (
    <div className="summary-screen">
      <Card tone="hero" raised className="summary-hero">
        <div className="summary-copy">
          <p className="eyebrow">{t("summary-eyebrow")}</p>
          <h1>{t("summary-title")}</h1>
          <p>{t("summary-body")}</p>
        </div>
        <div className="summary-sprite">
          <IellaSprite size={96} />
        </div>
      </Card>

      <div className="summary-grid">
        <Card tone="plain" className="metric-card">
          <p className="metric-label">{t("summary-accuracy")}</p>
          <p className="metric-value">{summary.accuracy}%</p>
          <ProgressBar value={summary.accuracy} tone="green" />
          <p className="metric-note">
            {t("summary-answers", {
              correct: summary.correct,
              total: summary.answered,
            })}
          </p>
        </Card>
        <Card tone="plain" className="metric-card">
          <p className="metric-label">{t("summary-duration")}</p>
          <p className="metric-value">
            {t("summary-minutes", { minutes })}
          </p>
          <p className="metric-note">{t("summary-duration-note")}</p>
        </Card>
        <Card tone="plain" className="metric-card">
          <p className="metric-label">{t("summary-mastered")}</p>
          <p className="metric-value">{summary.masteredKcs.length}</p>
          <p className="metric-note">
            {summary.masteredKcs.length > 0
              ? summary.masteredKcs.join(" · ")
              : t("summary-none-mastered")}
          </p>
        </Card>
      </div>

      <Card tone="tinted" className="summary-kcs">
        <div>
          <p className="eyebrow">{t("summary-practiced")}</p>
          <h2>{t("summary-practiced-title")}</h2>
        </div>
        <div className="summary-kc-list">
          {summary.kcTitles.length > 0 ? (
            summary.kcTitles.map((title) => <span key={title}>{title}</span>)
          ) : (
            <span>{t("summary-none-practiced")}</span>
          )}
        </div>
        {summary.openedKcs.length > 0 ? (
          <p className="summary-note">
            {t("summary-opened", { kcs: summary.openedKcs.join(" · ") })}
          </p>
        ) : null}
      </Card>

      <Button variant="primary" size="lg" block onClick={onDone}>
        {t("summary-done")}
      </Button>
    </div>
  );
}
