import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
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
  WidgetColSpan,
  WidgetId,
  WidgetRowSpan,
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
  updateWidgetSpan,
  widgetMinColSpans,
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

const WIDGET_LABEL_KEYS: Record<WidgetId, string> = {
  hero: "today-route-title",
  rules: "today-articles-title",
  memory: "today-memory-title",
  misconceptions: "today-focus-title",
  activity: "today-activity-title",
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
  const [editing, setEditing] = useState(false);
  const [draggingId, setDraggingId] = useState<WidgetId | null>(null);
  const [resizingId, setResizingId] = useState<WidgetId | null>(null);
  const [resizeSnap, setResizeSnap] = useState<{
    colSpan: WidgetColSpan;
    rowSpan: WidgetRowSpan;
  } | null>(null);
  const layoutRef = useRef(layout);
  const onLayoutChangeRef = useRef(onLayoutChange);
  layoutRef.current = layout;
  onLayoutChangeRef.current = onLayoutChange;
  const flipRef = useRef<Map<WidgetId, DOMRect> | null>(null);
  const dragRef = useRef<{
    id: WidgetId;
    pointerId: number;
    lastTarget: WidgetId;
  } | null>(null);
  const resizeRef = useRef<{
    id: WidgetId;
    pointerId: number;
    startX: number;
    startY: number;
    startColSpan: WidgetColSpan;
    startRowSpan: WidgetRowSpan;
    gridWidth: number;
  } | null>(null);
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

  const captureRects = () => {
    if (typeof document === "undefined") {
      return;
    }
    const next = new Map<WidgetId, DOMRect>();
    const nodes = document.querySelectorAll<HTMLElement>(
      ".dashboard-widget[data-widget]",
    );
    for (const node of nodes) {
      const id = node.dataset["widget"];
      if (id && layoutRef.current.order.includes(id as WidgetId)) {
        next.set(id as WidgetId, node.getBoundingClientRect());
      }
    }
    flipRef.current = next;
  };

  const changeOrder = (order: WidgetId[]) => {
    captureRects();
    onLayoutChange({ ...layoutRef.current, order });
  };

  const move = (id: WidgetId, direction: -1 | 1) => {
    changeOrder(moveWidget(layoutRef.current.order, id, direction));
  };

  const widgetAtPoint = (
    x: number,
    y: number,
    exclude: WidgetId,
  ): WidgetId | null => {
    const nodes = document.querySelectorAll<HTMLElement>(
      ".dashboard-widget[data-widget]",
    );
    let nearest: { id: WidgetId; distance: number } | null = null;
    for (const node of nodes) {
      const id = node.dataset["widget"];
      if (!id || id === exclude || !layoutRef.current.order.includes(id as WidgetId)) {
        continue;
      }
      const rect = node.getBoundingClientRect();
      if (
        x >= rect.left &&
        x <= rect.right &&
        y >= rect.top &&
        y <= rect.bottom
      ) {
        return id as WidgetId;
      }
      const centerX = rect.left + rect.width / 2;
      const centerY = rect.top + rect.height / 2;
      const distance = Math.hypot(x - centerX, y - centerY);
      if (distance < 180 && (!nearest || distance < nearest.distance)) {
        nearest = { id: id as WidgetId, distance };
      }
    }
    return nearest?.id ?? null;
  };

  const startDrag = (id: WidgetId, event: ReactPointerEvent<HTMLElement>) => {
    if (!editing) {
      return;
    }
    const target = event.target;
    if (
      target instanceof Element &&
      target.closest("button:not(.widget-drag-handle)")
    ) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    dragRef.current = {
      id,
      pointerId: event.pointerId,
      lastTarget: id,
    };
    setDraggingId(id);
    setResizingId(null);
    setResizeSnap(null);
    document.body.classList.add("widget-drag-active");
  };

  const startResize = (
    id: WidgetId,
    event: ReactPointerEvent<HTMLElement>,
  ) => {
    if (!editing) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const dashboard = event.currentTarget.closest(
      ".today-dashboard",
    ) as HTMLElement | null;
    if (!dashboard) {
      return;
    }
    resizeRef.current = {
      id,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startColSpan: layoutRef.current.colSpan[id],
      startRowSpan: layoutRef.current.rowSpan[id],
      gridWidth: dashboard.getBoundingClientRect().width,
    };
    setResizingId(id);
    setDraggingId(null);
    setResizeSnap({
      colSpan: layoutRef.current.colSpan[id],
      rowSpan: layoutRef.current.rowSpan[id],
    });
    document.body.classList.add("widget-resize-active");
  };

  useEffect(() => {
    if (!draggingId) {
      return;
    }
    const onMove = (event: PointerEvent) => {
      const current = dragRef.current;
      if (!current || event.pointerId !== current.pointerId) {
        return;
      }
      const target = widgetAtPoint(event.clientX, event.clientY, draggingId);
      if (!target || target === current.lastTarget) {
        return;
      }
      const layoutNow = layoutRef.current;
      const nextOrder = reorderWidget(layoutNow.order, draggingId, target);
      current.lastTarget = target;
      if (nextOrder.join("|") === layoutNow.order.join("|")) {
        return;
      }
      captureRects();
      layoutRef.current = { ...layoutNow, order: nextOrder };
      onLayoutChangeRef.current({ ...layoutNow, order: nextOrder });
    };
    const end = () => {
      dragRef.current = null;
      setDraggingId(null);
      document.body.classList.remove("widget-drag-active");
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      document.body.classList.remove("widget-drag-active");
    };
  }, [draggingId]);

  useEffect(() => {
    if (!resizingId) {
      return;
    }
    const onMove = (event: PointerEvent) => {
      const current = resizeRef.current;
      if (!current || event.pointerId !== current.pointerId) {
        return;
      }
      const gap = 14;
      const column = Math.max(80, (current.gridWidth - gap * 2) / 3);
      const deltaCols = Math.round((event.clientX - current.startX) / (column + gap));
      const nextCol = Math.max(
        widgetMinColSpans[current.id],
        Math.min(3, current.startColSpan + deltaCols),
      ) as WidgetColSpan;
      const deltaY = event.clientY - current.startY;
      const nextRow: WidgetRowSpan =
        deltaY > 84 ? "tall" : deltaY < -84 ? "compact" : current.startRowSpan;
      setResizeSnap({ colSpan: nextCol, rowSpan: nextRow });
      const layoutNow = layoutRef.current;
      if (
        layoutNow.colSpan[current.id] === nextCol &&
        layoutNow.rowSpan[current.id] === nextRow
      ) {
        return;
      }
      captureRects();
      const next = updateWidgetSpan(layoutNow, current.id, {
        colSpan: nextCol,
        rowSpan: nextRow,
      });
      layoutRef.current = next;
      onLayoutChangeRef.current(next);
    };
    const end = () => {
      resizeRef.current = null;
      setResizingId(null);
      setResizeSnap(null);
      document.body.classList.remove("widget-resize-active");
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
      document.body.classList.remove("widget-resize-active");
    };
  }, [resizingId]);

  useLayoutEffect(() => {
    const previous = flipRef.current;
    if (!previous) {
      return;
    }
    flipRef.current = null;
    if (
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
    ) {
      return;
    }
    const nodes = document.querySelectorAll<HTMLElement>(
      ".dashboard-widget[data-widget]",
    );
    for (const node of nodes) {
      const id = node.dataset["widget"] as WidgetId | undefined;
      if (!id || id === draggingId || id === resizingId) {
        continue;
      }
      const old = previous.get(id);
      if (!old) {
        continue;
      }
      const now = node.getBoundingClientRect();
      const dx = old.left - now.left;
      const dy = old.top - now.top;
      if (Math.abs(dx) < 1 && Math.abs(dy) < 1) {
        continue;
      }
      node.style.transition = "none";
      node.style.transform = `translate(${dx}px, ${dy}px)`;
      window.requestAnimationFrame(() => {
        node.style.transition = "transform 280ms cubic-bezier(0.2, 0.8, 0.2, 1)";
        node.style.transform = "";
        window.setTimeout(() => {
          node.style.transition = "";
        }, 300);
      });
    }
  }, [layout.order, layout.colSpan, layout.rowSpan, draggingId, resizingId]);

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
                <span className="memory-ring-particles" aria-hidden="true" />
                <span className="memory-ring-content">
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

  const widgetClass = (id: WidgetId): string =>
    [
      "dashboard-widget",
      `dashboard-widget-${id}`,
      id === "memory" || id === "misconceptions" || id === "activity"
        ? "dashboard-widget-telemetry"
        : "",
    ]
      .filter(Boolean)
      .join(" ");

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
                  setDraggingId(null);
                  setResizingId(null);
                  setResizeSnap(null);
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
                setDraggingId(null);
                setResizingId(null);
                setResizeSnap(null);
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
            data-col-span={layout.colSpan[id]}
            data-row-span={layout.rowSpan[id]}
            data-dragging={draggingId === id ? "true" : undefined}
            data-resizing={resizingId === id ? "true" : undefined}
            aria-label={t(WIDGET_LABEL_KEYS[id])}
          >
            {editing ? (
              <div
                className="widget-edit-bar"
                onPointerDown={(event) => startDrag(id, event)}
              >
                <span className="widget-drag-copy">
                  <button
                    type="button"
                    className="widget-drag-handle"
                    aria-label={t("today-widget-drag", {
                      title: t(WIDGET_LABEL_KEYS[id]),
                    })}
                    onPointerDown={(event) => {
                      event.stopPropagation();
                      startDrag(id, event);
                    }}
                  >
                    <StudioIcon name="grip" size={16} />
                  </button>
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
                  <span className="widget-span-chip" aria-hidden="true">
                    {layout.colSpan[id]} × {layout.rowSpan[id] === "tall"
                      ? t("today-widget-row-tall")
                      : t("today-widget-row-compact")}
                  </span>
                </span>
              </div>
            ) : null}
            {renderWidget(id)}
            {resizingId === id && resizeSnap ? (
              <span className="widget-ghost-target" aria-hidden="true" />
            ) : null}
            {editing ? (
              <button
                type="button"
                className="widget-resize-handle"
                aria-label={t("today-widget-resize", {
                  title: t(WIDGET_LABEL_KEYS[id]),
                })}
                onPointerDown={(event) => startResize(id, event)}
              >
                <span />
                <span />
                <span />
              </button>
            ) : null}
            {resizingId === id && resizeSnap ? (
              <span
                className="widget-resize-badge"
                role="status"
                aria-label={t("today-widget-resize-target", {
                  columns: resizeSnap.colSpan,
                  rows: resizeSnap.rowSpan === "tall" ? 2 : 1,
                })}
              >
                {resizeSnap.colSpan} × {resizeSnap.rowSpan === "tall" ? 2 : 1}
              </span>
            ) : null}
          </section>
        ))}
      </div>

      <Card tone="tinted" className="iella-note">
        <IellaSprite size={48} mood="curious" />
        <div>
          <strong>{t("iella-note-title")}</strong>
          <p>{t("iella-note-body")}</p>
        </div>
      </Card>

      <IellaTour open={tourOpen} onComplete={onTourComplete} />
    </div>
  );
}
