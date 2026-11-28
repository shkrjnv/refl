import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from "react";

import { IellaTip } from "../components/IellaTip";
import { NotesPanel } from "../components/NotesPanel";
import { StudioIcon } from "../components/StudioIcons";
import { Button, Card, StatusMeta } from "../components/ui";
import {
  CARD,
  edgeGeometry,
  estimateTextWidth,
  fitTransform,
  placeEdgeLabels,
  resolveGraphLayout,
  zoomAt,
  type GraphMode,
  type TextMeasure,
  type ViewTransform,
} from "../prototype/graphLayout";
import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import type { KcEntry, PrototypeContent } from "../prototype/content";
import {
  kcProgress,
  parseLearnerState,
  retrievability,
  type KcProgress,
} from "../prototype/progress";
import {
  parseRecords,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";

export interface GraphScreenProps {
  content: PrototypeContent;
  stateJson: string;
  locale: Locale;
  recordsWriter: RecordStore;
  onStateChange: (stateJson: string) => void;
  onPractice: (kcId: string) => void;
  onOpenNotes?: (noteId?: string) => void;
}

type NodeStatus = "not-started" | "learning" | "mastered" | "fading" | "blocked";

interface Node {
  entry: KcEntry;
  progress: KcProgress;
  status: NodeStatus;
  title: string;
}

const STATUS_TONE: Record<NodeStatus, "neutral" | "accent" | "success" | "danger"> = {
  "not-started": "neutral",
  learning: "accent",
  mastered: "success",
  fading: "danger",
  blocked: "neutral",
};

/** jsdom and the first paint have no width; lay out for a laptop until measured. */
const DEFAULT_VIEWPORT_WIDTH = 960;
const LAYERED_MAX_HEIGHT = 1200;
const DRAG_SLOP = 4;
const ZOOM_STEP = 1.2;

/** Card typography comes from the layout constants, so CSS cannot drift. */
const CARD_VARS = {
  "--graph-title-size": `${CARD.titleSize}px`,
  "--graph-title-weight": String(CARD.titleWeight),
  "--graph-title-line": String(CARD.titleLineHeight),
  "--graph-status-size": `${CARD.statusSize}px`,
  "--graph-status-weight": String(CARD.statusWeight),
  "--graph-status-line": String(CARD.statusLineHeight),
  "--graph-pad": `${CARD.padY}px ${CARD.padX}px`,
  "--graph-border": `${CARD.border}px`,
  "--graph-dot": `${CARD.dot}px`,
  "--graph-dot-gap": `${CARD.dotGap}px`,
  "--graph-row-gap": `${CARD.rowGap}px`,
} as unknown as CSSProperties;

/**
 * Canvas text metrics with the app font, when the browser has an
 * `OffscreenCanvas`. The estimate is the fallback and the floor for
 * unloaded fonts.
 */
function createMeasure(fontFamily: string): TextMeasure {
  if (typeof OffscreenCanvas === "undefined") {
    return estimateTextWidth;
  }
  const context = new OffscreenCanvas(1, 1).getContext("2d");
  if (!context) {
    return estimateTextWidth;
  }
  return (text, fontSize, weight) => {
    context.font = `${weight} ${fontSize}px ${fontFamily}`;
    return context.measureText(text).width * 1.04 + 1;
  };
}

function viewportHeightFor(mode: GraphMode, width: number): number {
  return mode === "web" ? Math.round(Math.max(380, Math.min(640, width * 0.62))) : 0;
}

export function GraphScreen({
  content,
  stateJson,
  locale,
  recordsWriter,
  onStateChange,
  onPractice,
  onOpenNotes,
}: GraphScreenProps) {
  const { t } = useTranslation();
  const learner = useMemo(() => parseLearnerState(stateJson), [stateJson]);
  const records: PersonalRecords = useMemo(
    () => parseRecords(stateJson),
    [stateJson],
  );
  const now = useMemo(() => Date.now(), [stateJson]);
  const progress = useMemo(
    () => kcProgress(content, learner, locale, now),
    [content, learner, locale, now],
  );
  const [mode, setMode] = useState<GraphMode>("layered");
  const [focused, setFocused] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [trail, setTrail] = useState<string[]>([]);
  const [noteOpen, setNoteOpen] = useState(false);
  const [viewportWidth, setViewportWidth] = useState(0);
  const [fontsVersion, setFontsVersion] = useState(0);
  const [view, setView] = useState<ViewTransform>({ scale: 1, x: 0, y: 0 });
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef<{
    startX: number;
    startY: number;
    view: ViewTransform;
    distance: number;
    dragging: boolean;
  } | null>(null);
  const suppressClick = useRef(false);

  const liveEntries = useMemo(() => content.liveKcs(), [content]);
  const liveIds = useMemo(
    () => new Set(liveEntries.map((entry) => entry.kc.id)),
    [liveEntries],
  );
  const edges = useMemo(
    () =>
      content.edges.filter(
        (edge) => liveIds.has(edge.from) && liveIds.has(edge.to),
      ),
    [content.edges, liveIds],
  );
  const nodes = useMemo<Node[]>(() => {
    return liveEntries.map((entry) => {
      const item = progress.find((candidate) => candidate.kcId === entry.kc.id);
      const state = learner.kcs[entry.kc.id];
      let status: NodeStatus = "not-started";
      if (state?.ever_mastered) {
        status = item?.due ? "fading" : "mastered";
      } else if ((state?.observations ?? 0) > 0) {
        status = "learning";
      } else {
        const blocked = content.edges.some(
          (edge) =>
            edge.kind === "requires" &&
            edge.from === entry.kc.id &&
            liveIds.has(edge.to) &&
            (learner.kcs[edge.to]?.observations ?? 0) === 0,
        );
        status = blocked ? "blocked" : "not-started";
      }
      return {
        entry,
        title: content.title(entry, locale),
        progress:
          item ?? {
            kcId: entry.kc.id,
            title: content.title(entry, locale),
            mastered: false,
            observations: 0,
            stabilityDays: 0,
            difficulty: 0,
            percent: 0,
            lastReview: null,
            due: false,
            level: entry.kc.level,
            domain: entry.kc.domain,
          },
        status,
      };
    });
  }, [liveEntries, progress, learner, content, liveIds, locale]);

  const measure = useMemo<TextMeasure>(() => {
    if (typeof document === "undefined") {
      return estimateTextWidth;
    }
    const family =
      getComputedStyle(document.documentElement)
        .getPropertyValue("--font-family")
        .trim() || "sans-serif";
    return createMeasure(family);
    // A font load changes metrics; `fontsVersion` re-creates the measurer.
  }, [fontsVersion]);

  const width = viewportWidth > 0 ? viewportWidth : DEFAULT_VIEWPORT_WIDTH;
  const webHeight = viewportHeightFor("web", width);
  const layout = useMemo(
    () =>
      resolveGraphLayout(
        nodes.map((node) => ({
          id: node.entry.kc.id,
          x: node.entry.layout_x,
          y: node.entry.layout_y,
          levelRank: node.entry.level_rank,
          level: node.entry.kc.level,
          domain: node.entry.kc.domain,
          title: node.title,
          statusLabel: t(`graph-status-${node.status}`),
        })),
        edges,
        content.contentVersion,
        mode,
        { width, height: webHeight },
        measure,
      ),
    [nodes, edges, content.contentVersion, mode, width, webHeight, measure, t],
  );

  const contrastLabel = t("graph-edge-contrasts");
  const edgeLabels = useMemo(() => {
    const labels = edges.flatMap((edge) => {
      const from = layout.boxes.get(edge.from);
      const to = layout.boxes.get(edge.to);
      if (edge.kind !== "contrasts" || !from || !to) {
        return [];
      }
      return [
        {
          id: `${edge.from}|${edge.to}`,
          at: edgeGeometry(from, to, mode).mid,
          width: Math.ceil(measure(contrastLabel, 11, 600)) + 10,
          height: 18,
        },
      ];
    });
    return placeEdgeLabels(layout.boxes, labels);
  }, [edges, layout, mode, measure, contrastLabel]);

  // Layered: full-size cards, page scrolls vertically. Web: a fixed stage
  // with the whole network fitted inside it.
  const layeredScale = Math.min(
    1,
    width / Math.max(1, layout.width),
    LAYERED_MAX_HEIGHT / Math.max(1, layout.height),
  );
  const stageHeight =
    mode === "web" ? webHeight : Math.ceil(layout.height * layeredScale);
  const fitted = useMemo(
    () => fitTransform(layout, { width, height: stageHeight }),
    [layout, width, stageHeight],
  );

  useLayoutEffect(() => {
    setView(fitted);
  }, [fitted]);

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }
    const update = () => {
      const next = Math.round(viewport.clientWidth);
      setViewportWidth((current) => (current === next ? current : next));
    };
    update();
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(viewport);
    return () => observer?.disconnect();
  }, []);

  useEffect(() => {
    let cancelled = false;
    void document.fonts?.ready
      .then(() => {
        if (!cancelled) {
          setFontsVersion((value) => value + 1);
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  // Ctrl/⌘ + wheel (and trackpad pinch) zooms; a plain wheel scrolls the page.
  useEffect(() => {
    const viewport = viewportRef.current;
    if (!viewport) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      if (!event.ctrlKey && !event.metaKey) {
        return;
      }
      event.preventDefault();
      const rect = viewport.getBoundingClientRect();
      const factor = Math.exp(-event.deltaY * 0.0025);
      setView((current) =>
        zoomAt(current, factor, {
          x: event.clientX - rect.left,
          y: event.clientY - rect.top,
        }),
      );
    };
    viewport.addEventListener("wheel", onWheel, { passive: false });
    return () => viewport.removeEventListener("wheel", onWheel);
  }, []);

  const pointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.pointerType === "mouse" && event.button !== 0) {
      return;
    }
    pointers.current.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const points = [...pointers.current.values()];
    const [first, second] = points;
    gesture.current = {
      startX: points.reduce((sum, point) => sum + point.x, 0) / points.length,
      startY: points.reduce((sum, point) => sum + point.y, 0) / points.length,
      view,
      distance: first && second ? Math.hypot(first.x - second.x, first.y - second.y) : 0,
      dragging: points.length > 1,
    };
  };

  const pointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!pointers.current.has(event.pointerId) || !gesture.current) {
      return;
    }
    pointers.current.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const points = [...pointers.current.values()];
    const centerX = points.reduce((sum, point) => sum + point.x, 0) / points.length;
    const centerY = points.reduce((sum, point) => sum + point.y, 0) / points.length;
    const state = gesture.current;
    const dx = centerX - state.startX;
    const dy = centerY - state.startY;
    if (!state.dragging && Math.hypot(dx, dy) < DRAG_SLOP) {
      return;
    }
    if (!state.dragging) {
      state.dragging = true;
      event.currentTarget.setPointerCapture?.(event.pointerId);
    }
    const [first, second] = points;
    let next: ViewTransform = {
      ...state.view,
      x: state.view.x + dx,
      y: state.view.y + dy,
    };
    if (first && second && state.distance > 0) {
      const rect = event.currentTarget.getBoundingClientRect();
      const factor = Math.hypot(first.x - second.x, first.y - second.y) / state.distance;
      next = zoomAt(next, factor, { x: centerX - rect.left, y: centerY - rect.top });
    }
    setView(next);
  };

  const pointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    pointers.current.delete(event.pointerId);
    if (gesture.current?.dragging) {
      suppressClick.current = true;
    }
    if (pointers.current.size === 0) {
      gesture.current = null;
    } else {
      // A finger lifted mid-pinch: continue as a pan from the remaining one.
      const [rest] = [...pointers.current.values()];
      gesture.current = rest
        ? { startX: rest.x, startY: rest.y, view, distance: 0, dragging: true }
        : null;
    }
  };

  const zoomBy = (factor: number) => {
    setView((current) =>
      zoomAt(current, factor, { x: width / 2, y: stageHeight / 2 }),
    );
  };

  const select = (id: string | null) => {
    if (id && selected && id !== selected) {
      setTrail((current) => [...current, selected].slice(-12));
    }
    if (!id) {
      setTrail([]);
    }
    setSelected(id);
    setNoteOpen(false);
  };

  const goBack = () => {
    const previous = trail[trail.length - 1];
    if (!previous) {
      return;
    }
    setTrail((current) => current.slice(0, -1));
    setSelected(previous);
    setNoteOpen(false);
  };

  const focus = selected ?? focused;
  const neighbors = useMemo(() => {
    if (!focus) {
      return null;
    }
    const set = new Set([focus]);
    for (const edge of edges) {
      if (edge.from === focus) {
        set.add(edge.to);
      }
      if (edge.to === focus) {
        set.add(edge.from);
      }
    }
    return set;
  }, [focus, edges]);
  const selectedNode = nodes.find((node) => node.entry.kc.id === selected) ?? null;
  const selectedState = selected ? learner.kcs[selected] : undefined;
  const previousNode = nodes.find(
    (node) => node.entry.kc.id === trail[trail.length - 1],
  );
  const misconceptions = Object.entries(learner.misconceptions)
    .map(([id, value]) => {
      const evidence = value.alpha + value.beta;
      return {
        id,
        strength: evidence > 0 ? value.alpha / evidence : 0,
        manifested: value.manifested,
      };
    })
    .filter((entry) => entry.manifested > 0)
    .sort((left, right) => right.strength - left.strength);

  return (
    <section
      className="screen graph-screen"
      aria-labelledby="graph-title"
      onKeyDown={(event) => {
        if (event.key === "Escape" && selected) {
          select(null);
        }
      }}
    >
      <header className="screen-head">
        <div>
          <p className="eyebrow">{t("graph-eyebrow")}</p>
          <h1 id="graph-title">{t("graph-title")}</h1>
          <p className="screen-note">{t("graph-note")}</p>
        </div>
        <div className="graph-mode" role="group" aria-label={t("graph-mode-label")}>
          <button
            type="button"
            className={`graph-mode-button${mode === "layered" ? " active" : ""}`}
            aria-pressed={mode === "layered"}
            onClick={() => setMode("layered")}
          >
            {t("graph-mode-layered")}
          </button>
          <button
            type="button"
            className={`graph-mode-button${mode === "web" ? " active" : ""}`}
            aria-pressed={mode === "web"}
            onClick={() => setMode("web")}
          >
            {t("graph-mode-web")}
          </button>
        </div>
      </header>
      {layout.fallback ? (
        <p className="graph-note">{t("graph-fallback-note")}</p>
      ) : null}
      <div className={`graph-canvas graph-${mode}`} style={CARD_VARS}>
        <div
          ref={viewportRef}
          className="graph-viewport"
          style={{ height: `${stageHeight}px` }}
          onPointerDown={pointerDown}
          onPointerMove={pointerMove}
          onPointerUp={pointerUp}
          onPointerCancel={pointerUp}
          onClickCapture={(event) => {
            if (suppressClick.current) {
              suppressClick.current = false;
              event.preventDefault();
              event.stopPropagation();
            }
          }}
        >
          <div
            className="graph-world"
            style={{
              width: `${layout.width}px`,
              height: `${layout.height}px`,
              transform: `translate(${view.x}px, ${view.y}px) scale(${view.scale})`,
            }}
          >
            <svg
              width={layout.width}
              height={layout.height}
              viewBox={`0 0 ${layout.width} ${layout.height}`}
              role="img"
              aria-label={t("graph-canvas-label")}
              className="graph-svg"
            >
              <defs>
                <marker
                  id="graph-arrow"
                  viewBox="0 0 10 10"
                  refX="9"
                  refY="5"
                  markerWidth="7"
                  markerHeight="7"
                  markerUnits="userSpaceOnUse"
                  orient="auto"
                >
                  <path d="M 0 0 L 10 5 L 0 10 z" className="graph-arrow-head" />
                </marker>
              </defs>
              {edges.map((edge) => {
                // `requires` points from the prerequisite to the topic that needs it.
                const fromId = edge.kind === "requires" ? edge.to : edge.from;
                const toId = edge.kind === "requires" ? edge.from : edge.to;
                const from = layout.boxes.get(fromId);
                const to = layout.boxes.get(toId);
                if (!from || !to) {
                  return null;
                }
                const geometry = edgeGeometry(from, to, mode);
                const dim =
                  neighbors !== null &&
                  !(neighbors.has(edge.from) && neighbors.has(edge.to));
                return (
                  <g
                    key={`${edge.kind}-${edge.from}-${edge.to}`}
                    className={`graph-edge-group${dim ? " dim" : ""}`}
                  >
                    <path
                      d={geometry.d}
                      className={`graph-edge graph-edge-${edge.kind}`}
                      markerEnd={
                        edge.kind === "requires" ? "url(#graph-arrow)" : undefined
                      }
                    />
                    {edge.kind === "contrasts" ? (
                      <text
                        x={edgeLabels.get(`${edge.from}|${edge.to}`)?.x ?? geometry.mid.x}
                        y={edgeLabels.get(`${edge.from}|${edge.to}`)?.y ?? geometry.mid.y}
                        className="graph-edge-label"
                        textAnchor="middle"
                        dominantBaseline="central"
                      >
                        {contrastLabel}
                      </text>
                    ) : null}
                  </g>
                );
              })}
            </svg>
            {nodes.map((node) => {
              const id = node.entry.kc.id;
              const box = layout.boxes.get(id);
              if (!box) {
                return null;
              }
              const dim = neighbors !== null && !neighbors.has(id);
              return (
                <button
                  key={id}
                  type="button"
                  className={`graph-node graph-node-${node.status}${dim ? " dim" : ""}${selected === id ? " selected" : ""}`}
                  style={{
                    left: `${box.x}px`,
                    top: `${box.y}px`,
                    width: `${box.width}px`,
                    height: `${box.height}px`,
                  }}
                  aria-pressed={selected === id}
                  aria-label={t("graph-node-label", {
                    title: node.title,
                    status: t(`graph-status-${node.status}`),
                  })}
                  onPointerEnter={(event) => {
                    if (event.pointerType === "mouse") {
                      setFocused(id);
                    }
                  }}
                  onPointerLeave={() => setFocused(null)}
                  onFocus={() => setFocused(id)}
                  onBlur={() => setFocused(null)}
                  onClick={() => select(id)}
                >
                  <span className="graph-node-head">
                    <span className="graph-node-title">{node.title}</span>
                    <span className="graph-node-dot" aria-hidden="true" />
                  </span>
                  <span className="graph-node-status">
                    {t(`graph-status-${node.status}`)}
                  </span>
                </button>
              );
            })}
          </div>
          {nodes.length === 0 ? (
            <p className="bento-empty graph-empty">{t("graph-empty")}</p>
          ) : null}
        </div>
        <div className="graph-toolbar">
          <ul className="graph-legend" aria-label={t("graph-legend-label")}>
            <li>
              <svg width="28" height="10" aria-hidden="true">
                <path d="M 2 5 L 24 5" className="graph-edge graph-edge-requires" markerEnd="url(#graph-arrow)" />
              </svg>
              {t("graph-legend-requires")}
            </li>
            <li>
              <svg width="28" height="10" aria-hidden="true">
                <path d="M 2 5 L 26 5" className="graph-edge graph-edge-supports" />
              </svg>
              {t("graph-legend-supports")}
            </li>
            <li>
              <svg width="28" height="10" aria-hidden="true">
                <path d="M 2 5 L 26 5" className="graph-edge graph-edge-contrasts" />
              </svg>
              {t("graph-legend-contrasts")}
            </li>
          </ul>
          <div className="graph-zoom" role="group" aria-label={t("graph-zoom-label")}>
            <button
              type="button"
              className="graph-zoom-button"
              aria-label={t("graph-zoom-out")}
              title={t("graph-zoom-out")}
              onClick={() => zoomBy(1 / ZOOM_STEP)}
            >
              <StudioIcon name="minus" size={16} />
            </button>
            <button
              type="button"
              className="graph-zoom-button graph-zoom-fit"
              onClick={() => setView(fitted)}
            >
              {t("graph-zoom-fit")}
            </button>
            <button
              type="button"
              className="graph-zoom-button"
              aria-label={t("graph-zoom-in")}
              title={t("graph-zoom-in")}
              onClick={() => zoomBy(ZOOM_STEP)}
            >
              <StudioIcon name="plus" size={16} />
            </button>
          </div>
        </div>
        <p className="graph-gesture-hint">{t("graph-gesture-hint")}</p>
      </div>
      {selectedNode ? (
        <Card tone="plain" className="graph-drawer">
          <header className="graph-drawer-head">
            <div>
              <p className="eyebrow">
                {selectedNode.entry.kc.level} · {selectedNode.entry.kc.domain}
              </p>
              <h2>{selectedNode.title}</h2>
            </div>
            <StatusMeta tone={STATUS_TONE[selectedNode.status]}>
              {t(`graph-status-${selectedNode.status}`)}
            </StatusMeta>
          </header>
          {previousNode ? (
            <button type="button" className="graph-back" onClick={goBack}>
              <StudioIcon name="arrow-left" size={14} />
              {t("graph-back", { title: previousNode.title })}
            </button>
          ) : null}
          <div className="graph-facts">
            <div>
              <span>{t("graph-fact-observations")}</span>
              <strong>{selectedState?.observations ?? 0}</strong>
            </div>
            <div>
              <span>{t("graph-fact-stability")}</span>
              <strong>
                {(selectedState?.card?.stability ?? 0).toFixed(1)}d
              </strong>
            </div>
            <div>
              <span>{t("graph-fact-recall")}</span>
              <strong>
                {selectedState?.card
                  ? `${Math.round(
                      retrievability(
                        selectedState.card.stability,
                        selectedState.card.last_review,
                        now,
                      ) * 100,
                    )}%`
                  : "—"}
              </strong>
            </div>
            <div>
              <span>{t("graph-fact-due")}</span>
              <strong>
                {selectedNode.progress.due ? t("graph-fact-due-now") : t("graph-fact-not-due")}
              </strong>
            </div>
          </div>
          <details className="graph-why">
            <summary>{t("graph-why-title")}</summary>
            <p>
              {t("graph-why-body", {
                observations: selectedState?.observations ?? 0,
                stability: (selectedState?.card?.stability ?? 0).toFixed(1),
              })}
            </p>
            {misconceptions.length > 0 ? (
              <p>
                {t("graph-why-misconceptions", {
                  count: misconceptions.length,
                })}
              </p>
            ) : null}
          </details>
          <div className="graph-actions">
            <Button type="button" variant="primary" onClick={() => onPractice(selectedNode.entry.kc.id)}>
              {t("graph-practice")}
            </Button>
            <Button
              type="button"
              variant="secondary"
              onClick={() => setNoteOpen((open) => !open)}
            >
              {t("graph-note-action")}
            </Button>
          </div>
          {noteOpen ? (
            <NotesPanel
              records={records}
              anchor={{ kind: "kc", id: selectedNode.entry.kc.id }}
              recordsWriter={recordsWriter}
              stateJson={stateJson}
              onStateChange={onStateChange}
              title={t("graph-note-title")}
              onOpenWorkspace={onOpenNotes}
              onClose={() => setNoteOpen(false)}
            />
          ) : null}
        </Card>
      ) : (
        <p className="bento-note">{t("graph-select-hint")}</p>
      )}
      <IellaTip section="graph" />
    </section>
  );
}
