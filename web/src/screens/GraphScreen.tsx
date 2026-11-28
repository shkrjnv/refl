import { useMemo, useState } from "react";

import { IellaTip } from "../components/IellaTip";
import { NotesPanel } from "../components/NotesPanel";
import { Button, Card, StatusMeta } from "../components/ui";
import { resolveGraphPositions } from "../prototype/graphLayout";
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

type GraphMode = "layered" | "web";
type NodeStatus = "not-started" | "learning" | "mastered" | "fading" | "blocked";

interface Node {
  entry: KcEntry;
  progress: KcProgress;
  status: NodeStatus;
  x: number;
  y: number;
}

const STATUS_TONE: Record<NodeStatus, "neutral" | "accent" | "success" | "danger"> = {
  "not-started": "neutral",
  learning: "accent",
  mastered: "success",
  fading: "danger",
  blocked: "neutral",
};

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
  const now = Date.now();
  const progress = useMemo(
    () => kcProgress(content, learner, locale, now),
    [content, learner, locale, now],
  );
  const [mode, setMode] = useState<GraphMode>("layered");
  const [focused, setFocused] = useState<string | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [noteOpen, setNoteOpen] = useState(false);

  const liveEntries = content.liveKcs();
  const liveIds = new Set(liveEntries.map((entry) => entry.kc.id));
  const edges = content.edges.filter(
    (edge) => liveIds.has(edge.from) && liveIds.has(edge.to),
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
        x: 80 + (entry.layout_x / 1000) * 840,
        y: 80 + (entry.layout_y / 1000) * 840,
      };
    });
  }, [liveEntries, progress, learner, content, liveIds, locale]);

  const resolved = useMemo(
    () =>
      resolveGraphPositions(
        nodes.map((node) => ({
          id: node.entry.kc.id,
          x: node.entry.layout_x,
          y: node.entry.layout_y,
          levelRank: node.entry.level_rank,
          level: node.entry.kc.level,
          domain: node.entry.kc.domain,
        })),
        edges,
        content.contentVersion,
        mode,
      ),
    [mode, nodes, edges, content.contentVersion],
  );
  const positions = resolved.positions;

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
    <section className="screen graph-screen" aria-labelledby="graph-title">
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
            className={`graph-mode-button graph-web-toggle${mode === "web" ? " active" : ""}`}
            aria-pressed={mode === "web"}
            onClick={() => setMode("web")}
          >
            {t("graph-mode-web")}
          </button>
        </div>
      </header>
      {resolved.fallback ? (
        <p className="graph-note">{t("graph-fallback-note")}</p>
      ) : null}
      <div className={`graph-canvas graph-${mode}`}>
        <div className="graph-stage">
          <svg
            viewBox="0 0 1000 1000"
            preserveAspectRatio="none"
            role="img"
            aria-label={t("graph-canvas-label")}
            className="graph-svg"
          >
            <defs>
              <marker
                id="graph-arrow"
                viewBox="0 0 10 10"
                refX="8"
                refY="5"
                markerWidth="6"
                markerHeight="6"
                orient="auto-start-reverse"
              >
                <path d="M 0 0 L 10 5 L 0 10 z" className="graph-arrow-head" />
              </marker>
            </defs>
            {edges.map((edge) => {
              const left = positions.get(edge.from);
              const right = positions.get(edge.to);
              if (!left || !right) {
                return null;
              }
              const dim =
                neighbors !== null &&
                !(neighbors.has(edge.from) && neighbors.has(edge.to));
              return (
                <line
                  key={`${edge.kind}-${edge.from}-${edge.to}`}
                  x1={left.x}
                  y1={left.y}
                  x2={right.x}
                  y2={right.y}
                  className={`graph-edge graph-edge-${edge.kind}${dim ? " dim" : ""}`}
                  markerEnd={edge.kind === "requires" ? "url(#graph-arrow)" : undefined}
                />
              );
            })}
          </svg>
          {nodes.map((node) => {
            const position = positions.get(node.entry.kc.id) ?? { x: 500, y: 500 };
            const dim = neighbors !== null && !neighbors.has(node.entry.kc.id);
            const title = content.title(node.entry, locale);
            return (
              <button
                key={node.entry.kc.id}
                type="button"
                className={`graph-node graph-node-${node.status}${dim ? " dim" : ""}${selected === node.entry.kc.id ? " selected" : ""}`}
                style={{
                  left: `${position.x / 10}%`,
                  top: `${position.y / 10}%`,
                }}
                aria-pressed={selected === node.entry.kc.id}
                aria-label={t("graph-node-label", {
                  title,
                  status: t(`graph-status-${node.status}`),
                })}
                onPointerEnter={() => setFocused(node.entry.kc.id)}
                onPointerLeave={() => setFocused(null)}
                onFocus={() => setFocused(node.entry.kc.id)}
                onBlur={() => setFocused(null)}
                onClick={() => {
                  setSelected(node.entry.kc.id);
                  setNoteOpen(false);
                }}
              >
                <span className="graph-node-dot" aria-hidden="true" />
                <span className="graph-node-title">{title}</span>
                <span className="graph-node-status">
                  {t(`graph-status-${node.status}`)}
                </span>
              </button>
            );
          })}
          {nodes.length === 0 ? (
            <p className="bento-empty">{t("graph-empty")}</p>
          ) : null}
        </div>
      </div>
      {selectedNode ? (
        <Card tone="plain" className="graph-drawer">
          <header className="graph-drawer-head">
            <div>
              <p className="eyebrow">
                {selectedNode.entry.kc.level} · {selectedNode.entry.kc.domain}
              </p>
              <h2>{content.title(selectedNode.entry, locale)}</h2>
            </div>
            <StatusMeta tone={STATUS_TONE[selectedNode.status]}>
              {t(`graph-status-${selectedNode.status}`)}
            </StatusMeta>
          </header>
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
                  count: misconceptions.filter((entry) => entry.manifested > 0).length,
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
