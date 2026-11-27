import {
  useMemo,
  useState,
  type CSSProperties,
  type DragEvent,
  type ReactNode,
} from "react";

import { HeroIllustration } from "../components/HeroIllustration";
import { IellaTour } from "../components/IellaTour";
import { StudioIcon } from "../components/StudioIcons";
import { IellaSprite } from "../components/IellaSprite";
import { Button, Card, ProgressBar } from "../components/ui";
import { useTranslation } from "../i18n";
import type { ActivitySummary } from "../prototype/activity";
import type { PrototypeContent } from "../prototype/content";
import type {
  DashboardLayout,
  WidgetId,
  WidgetSize,
} from "../prototype/layout";
import {
  dueCount,
  kcProgress,
  memoryRetention,
  parseLearnerState,
  statsSummary,
  weakestKcs,
} from "../prototype/progress";
import type { AppStatus } from "../prototype/status";
import {
  moveWidget,
  reorderWidget,
  toggleWidgetSize,
} from "../prototype/layout";

export interface TodayScreenProps {
  content: PrototypeContent;
  stateJson: string | null;
  learnerLabel: string;
  streak: number;
  status: AppStatus;
  activity: ActivitySummary;
  layout: DashboardLayout;
  onLayoutChange: (layout: DashboardLayout) => void;
  onResetLayout: () => void;
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

const WIDGET_LABEL_KEYS: Record<WidgetId, string> = {
  hero: "today-route-title",
  rules: "today-articles-title",
  memory: "today-memory-title",
  misconceptions: "today-focus-title",
  activity: "today-activity-title",
};

const SIZE_SUPPORTED: Record<WidgetId, boolean> = {
  hero: false,
  rules: false,
  memory: true,
  misconceptions: true,
  activity: true,
};

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

function isTelemetryWidget(id: WidgetId): boolean {
  return id === "memory" || id === "misconceptions" || id === "activity";
}

export function TodayScreen({
  content,
  stateJson,
  learnerLabel,
  streak,
  status,
  activity,
  layout,
  onLayoutChange,
  onResetLayout,
  onStart,
  onOpenStats,
  tourOpen,
  onTourComplete,
}: TodayScreenProps) {
  const { t, locale } = useTranslation();
  const [tipIndex, setTipIndex] = useState<number | null>(null);
  const [editing, setEditing] = useState(false);
  const [dragging, setDragging] = useState<WidgetId | null>(null);
  const progress = useMemo(
    () => kcProgress(content, parseLearnerState(stateJson), locale),
    [content, stateJson, locale],
  );
  const stats = statsSummary(progress);
  const retention = memoryRetention(progress);
  const due = dueCount(progress);
  const weak = weakestKcs(progress);
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

  const nextTip = () => {
    setTipIndex((current) =>
      current === null ? 0 : (current + 1) % IELLA_TIPS.length,
    );
  };

  const changeOrder = (order: WidgetId[]) => {
    onLayoutChange({ ...layout, order });
  };

  const move = (id: WidgetId, direction: -1 | 1) => {
    changeOrder(moveWidget(layout.order, id, direction));
  };

  const toggleSize = (id: WidgetId) => {
    onLayoutChange({
      ...layout,
      sizes: toggleWidgetSize(layout.sizes, id),
    });
  };

  const onDragStart = (id: WidgetId, event: DragEvent<HTMLElement>) => {
    if (!editing) {
      event.preventDefault();
      return;
    }
    setDragging(id);
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", id);
  };

  const onDrop = (target: WidgetId, event: DragEvent<HTMLElement>) => {
    event.preventDefault();
    const source =
      dragging ??
      (event.dataTransfer.getData("text/plain") as WidgetId | null);
    setDragging(null);
    if (!source || !layout.order.includes(source)) {
      return;
    }
    changeOrder(reorderWidget(layout.order, source, target));
  };

  const ringStyle = {
    "--ring-angle": `${retention * 3.6}deg`,
  } as CSSProperties;

  const renderWidget = (id: WidgetId): ReactNode => {
    switch (id) {
      case "hero":
        return (
          <section
            className="today-route"
            data-tour="route"
            aria-labelledby="today-route-title"
          >
            <HeroIllustration />
            <div className="today-route-perch">
              <IellaSprite
                size={60}
                label={t("iella-tip-open")}
                onActivate={nextTip}
              />
            </div>
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
        );
      case "rules":
        return (
          <section
            className="articles-block"
            aria-labelledby="today-articles-title"
          >
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
        );
      case "memory":
        return (
          <Card tone="plain" className="telemetry-card memory-card">
            <div className="telemetry-heading">
              <div>
                <span className="telemetry-kicker">{t("stats-memory")}</span>
                <h2>{t("today-memory-title")}</h2>
              </div>
              <span className="telemetry-due">
                {t("today-memory-due", { count: due })}
              </span>
            </div>
            <div className="memory-ring-row">
              <div
                className="memory-ring"
                style={ringStyle}
                role="img"
                aria-label={t("today-memory-ring", {
                  percent: retention,
                  count: stats.inMemory,
                })}
              >
                <span>
                  <strong>{retention}%</strong>
                  <small>{t("today-memory-in-memory")}</small>
                </span>
              </div>
              <div className="memory-copy">
                <span>{t("today-memory-count", {
                  count: stats.inMemory,
                  total: progress.length,
                })}</span>
                <p>
                  {stats.inMemory === 0
                    ? t("today-memory-empty")
                    : t("today-memory-stability", {
                        days: Math.max(1, Math.round(stats.averageStability)),
                      })}
                </p>
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  disabled={due === 0}
                  onClick={onStart}
                >
                  {t("today-review-due")}
                </Button>
              </div>
            </div>
          </Card>
        );
      case "misconceptions":
        return (
          <Card tone="plain" className="telemetry-card focus-card">
            <div className="telemetry-heading">
              <div>
                <span className="telemetry-kicker">{t("today-focus-kicker")}</span>
                <h2>{t("today-focus-title")}</h2>
              </div>
              <Button type="button" variant="ghost" size="sm" onClick={onOpenStats}>
                {t("today-open-stats")}
              </Button>
            </div>
            {weak.length === 0 ? (
              <p className="telemetry-empty">{t("today-focus-empty")}</p>
            ) : (
              <div className="focus-list">
                {weak.map((kc) => (
                  <div className="focus-row" key={kc.kcId}>
                    <div className="focus-label">
                      <span>{kc.title}</span>
                      <span>{kc.percent}%</span>
                    </div>
                    <ProgressBar
                      value={kc.percent}
                      tone="coral"
                      label={kc.title}
                    />
                  </div>
                ))}
              </div>
            )}
          </Card>
        );
      case "activity":
        return (
          <Card tone="plain" className="telemetry-card activity-card">
            <div className="telemetry-heading">
              <div>
                <span className="telemetry-kicker">{t("today-activity-kicker")}</span>
                <h2>{t("today-activity-title")}</h2>
              </div>
              <span className="activity-best">
                {t("today-activity-best", { days: activity.bestStreak })}
              </span>
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
                  aria-hidden="true"
                />
              ))}
            </div>
            <p className="activity-note">
              {activity.total === 0
                ? t("today-activity-empty")
                : t("today-activity-today", { count: activity.todayCount })}
            </p>
          </Card>
        );
      default:
        return null;
    }
  };

  const widgetClass = (id: WidgetId): string => {
    const size = layout.sizes[id];
    return [
      "dashboard-widget",
      `dashboard-widget-${id}`,
      isTelemetryWidget(id) ? "dashboard-widget-telemetry" : "",
      `widget-size-${size}`,
    ]
      .filter(Boolean)
      .join(" ");
  };

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
          <div className="today-layout-actions">
            {editing ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => {
                  setDragging(null);
                  onResetLayout();
                }}
              >
                {t("today-reset-layout")}
              </Button>
            ) : null}
            <Button
              type="button"
              variant={editing ? "primary" : "secondary"}
              size="sm"
              aria-pressed={editing}
              onClick={() => {
                setEditing((value) => !value);
                setDragging(null);
              }}
            >
              <StudioIcon name="grip" size={16} />
              {editing ? t("today-done") : t("today-edit-layout")}
            </Button>
          </div>
        </div>
      </header>

      {status.updateReady ? (
        <div className="update-banner" role="status">
          <span>{t("sync-update-ready")}</span>
          <span>{t("sync-update-deferred")}</span>
        </div>
      ) : null}

      <div className={`today-dashboard${editing ? " editing" : ""}`}>
        {layout.order.map((id, index) => (
          <section
            key={id}
            className={widgetClass(id)}
            data-widget={id}
            data-size={layout.sizes[id] satisfies WidgetSize}
            draggable={editing}
            onDragStart={(event) => onDragStart(id, event)}
            onDragOver={(event) => {
              if (editing && dragging && dragging !== id) {
                event.preventDefault();
                event.dataTransfer.dropEffect = "move";
              }
            }}
            onDrop={(event) => onDrop(id, event)}
            onDragEnd={() => setDragging(null)}
            aria-label={t(WIDGET_LABEL_KEYS[id])}
          >
            {editing ? (
              <div className="widget-edit-bar">
                <span className="widget-drag-copy">
                  <span className="widget-drag-handle" aria-hidden="true">
                    <StudioIcon name="grip" size={16} />
                  </span>
                  <span>{t(WIDGET_LABEL_KEYS[id])}</span>
                </span>
                <span className="widget-edit-actions">
                  <button
                    type="button"
                    className="widget-icon-button"
                    aria-label={t("today-widget-move-up", {
                      title: t(WIDGET_LABEL_KEYS[id]),
                    })}
                    disabled={index === 0}
                    onClick={() => move(id, -1)}
                  >
                    <StudioIcon name="arrow-up" size={15} />
                  </button>
                  <button
                    type="button"
                    className="widget-icon-button"
                    aria-label={t("today-widget-move-down", {
                      title: t(WIDGET_LABEL_KEYS[id]),
                    })}
                    disabled={index === layout.order.length - 1}
                    onClick={() => move(id, 1)}
                  >
                    <StudioIcon name="arrow-down" size={15} />
                  </button>
                  {SIZE_SUPPORTED[id] ? (
                    <button
                      type="button"
                      className="widget-size-button"
                      aria-pressed={layout.sizes[id] === "wide"}
                      onClick={() => toggleSize(id)}
                    >
                      {layout.sizes[id] === "wide"
                        ? t("today-widget-compact")
                        : t("today-widget-wide")}
                    </button>
                  ) : null}
                </span>
              </div>
            ) : null}
            {renderWidget(id)}
          </section>
        ))}
      </div>

      <Card tone="tinted" className="iella-note">
        <button
          type="button"
          className="iella-note-icon"
          aria-label={t("iella-tip-open")}
          onClick={nextTip}
        >
          <StudioIcon name="spark" size={22} />
        </button>
        <div>
          <strong>{t("iella-note-title")}</strong>
          <p aria-live="polite">{tip ?? t("iella-note-body")}</p>
        </div>
      </Card>

      <IellaTour open={tourOpen} onComplete={onTourComplete} />
    </div>
  );
}
