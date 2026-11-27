import { useMemo } from "react";

import { StudioIcon } from "../components/StudioIcons";
import { Badge, Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import type { PrototypeContent } from "../prototype/content";
import {
  dueCount,
  kcProgress,
  memoryRetention,
  parseLearnerState,
  statsSummary,
  totalReviews,
} from "../prototype/progress";

export interface ProfileScreenProps {
  learnerLabel: string;
  learnerId: string;
  authMethod: "email" | "telegram";
  content: PrototypeContent;
  stateJson: string | null;
  streak: number;
  isAdmin: boolean;
  onOpenSettings: () => void;
  onLogout: () => void;
}

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
  onOpenSettings,
  onLogout,
}: ProfileScreenProps) {
  const { t, locale } = useTranslation();
  const progress = useMemo(
    () => kcProgress(content, parseLearnerState(stateJson), locale),
    [content, stateJson, locale],
  );
  const stats = statsSummary(progress);
  const reviews = totalReviews(progress);
  const due = dueCount(progress);
  const retention = memoryRetention(progress);
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

  const methodRow = (kind: "email" | "telegram", labelKey: string) => {
    const linked = authMethod === kind;
    return (
      <div className="method-row" key={kind}>
        <span>{t(labelKey)}</span>
        <Badge tone={linked ? "green" : "neutral"}>
          {linked ? t("settings-linked") : t("settings-not-linked")}
        </Badge>
      </div>
    );
  };

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
            <code>{learnerId}</code>
            <div className="profile-pills">
              <Badge tone={isAdmin ? "sun" : "neutral"}>
                {isAdmin ? t("profile-role-admin") : t("profile-role-learner")}
              </Badge>
              <Badge tone="green">{t("profile-local")}</Badge>
            </div>
          </div>
        </div>
        <div className="profile-heading-actions">
          <span className="streak-badge">
            <StudioIcon name="streak" size={17} />
            <span>{t("today-streak", { days: streak })}</span>
          </span>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            onClick={onOpenSettings}
          >
            <StudioIcon name="gear" size={16} />
            {t("common-settings")}
          </Button>
        </div>
      </header>

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
          <span className="profile-stat-inline">
            {t("profile-retention", { percent: retention })}
          </span>
        </Card>

        <Card tone="plain" className="profile-stat-card">
          <span className="profile-stat-label">{t("profile-reviews")}</span>
          <strong className="profile-stat-value">{reviews}</strong>
          <span className="profile-stat-note">{t("profile-reviews-note")}</span>
          <div className="profile-review-parts">
            <span>
              <strong>{streak}</strong>
              {t("profile-streak-short")}
            </span>
            <span>
              <strong>{due}</strong>
              {t("today-memory-due-label")}
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

      <section className="profile-section" aria-labelledby="profile-account-title">
        <div className="profile-section-heading">
          <div>
            <h2 id="profile-account-title">{t("profile-account-title")}</h2>
            <p>{t("settings-method-note")}</p>
          </div>
        </div>
        <div className="settings-two-column">
          <Card tone="plain" className="settings-card">
            <h3>{t("profile-linked")}</h3>
            <div className="method-list">
              {methodRow("email", "settings-method-email")}
              {methodRow("telegram", "settings-method-telegram")}
            </div>
          </Card>
          <Card tone="plain" className="settings-card settings-split-card">
            <div>
              <h3>{t("auth-sign-out")}</h3>
              <p className="settings-note">{t("profile-logout-note")}</p>
            </div>
            <Button type="button" variant="danger" size="sm" onClick={onLogout}>
              {t("auth-sign-out")}
            </Button>
          </Card>
        </div>
      </section>

      <Card tone="tinted" className="profile-route-card">
        <div>
          <h2>{t("profile-route-title")}</h2>
          <p>{t("profile-route-note")}</p>
        </div>
        <Button type="button" variant="secondary" size="sm" onClick={onOpenSettings}>
          <StudioIcon name="gear" size={16} />
          {t("settings-tab-appearance")}
        </Button>
      </Card>
    </div>
  );
}
