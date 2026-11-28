import { useMemo, useState } from "react";

import { IellaTip } from "../components/IellaTip";
import { NotesPanel } from "../components/NotesPanel";
import { Button, Card, StatusMeta } from "../components/ui";
import type { Edge } from "../generated/core/Edge";
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

function seedFrom(text: string): number {
  let seed = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    seed ^= text.charCodeAt(index);
    seed = Math.imul(seed, 16777619);
  }
  return seed >>> 0;
}

function random(seed: number): () => number {
  let value = seed;
  return () => {
    value += 0x6d2b79f5;
    let t = value;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Deterministic force layout: 300 steps, seeded by the content version. */
function webPositions(
  nodes: Node[],
  edges: Edge[],
  seed: number,
): Map<string, { x: number; y: number }> {
  const next = random(seed);
  const positions = new Map(
    nodes.map((node) => [
      node.entry.kc.id,
      { x: node.x + next() * 40, y: node.y + next() * 40 },
    ]),
  );
  const ids = nodes.map((node) => node.entry.kc.id);
  const links = edges.filter(
    (edge) => positions.has(edge.from) && positions.has(edge.to),
  );
  for (let step = 0; step < 300; step += 1) {
    const cooling = 1 - step / 300;
    for (let i = 0; i < ids.length; i += 1) {
      const left = positions.get(ids[i] ?? "");
      if (!left) {
        continue;
      }
      for (let j = i + 1; j < ids.length; j += 1) {
        const right = positions.get(ids[j] ?? "");
        if (!right) {
          continue;
        }
        let dx = left.x - right.x;
        let dy = left.y - right.y;
        const distance = Math.max(1, Math.hypot(dx, dy));
        const force = 9000 / (distance * distance);
        dx /= distance;
        dy /= distance;
        left.x += dx * force * cooling;
        left.y += dy * force * cooling;
        right.x -= dx * force * cooling;
        right.y -= dy * force * cooling;
      }
    }
    for (const edge of links) {
      const left = positions.get(edge.from);
      const right = positions.get(edge.to);
      if (!left || !right) {
        continue;
      }
      const dx = right.x - left.x;
      const dy = right.y - left.y;
      const distance = Math.max(1, Math.hypot(dx, dy));
      const force = (distance - 220) * 0.004 * cooling;
      const ux = dx / distance;
      const uy = dy / distance;
      left.x += ux * force;
      left.y += uy * force;
      right.x -= ux * force;
      right.y -= uy * force;
    }
    for (const id of ids) {
      const position = positions.get(id);
      if (!position) {
        continue;
      }
      position.x = Math.max(80, Math.min(920, position.x * 0.995 + 500 * 0.005));
      position.y = Math.max(80, Math.min(920, position.y * 0.995 + 500 * 0.005));
    }
  }
  return positions;
}

export function GraphScreen({
  content,
  stateJson,
  locale,
  recordsWriter,
  onStateChange,
  onPractice,
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

  const positions = useMemo(() => {
    if (mode === "layered") {
      return new Map(nodes.map((node) => [node.entry.kc.id, { x: node.x, y: node.y }]));
    }
    return webPositions(nodes, edges, seedFrom(content.contentVersion));
  }, [mode, nodes, edges, content.contentVersion]);

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
      <div className={`graph-canvas graph-${mode}`}>
        <svg
          viewBox="0 0 1000 1000"
          role="group"
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
          {nodes.map((node) => {
            const position = positions.get(node.entry.kc.id) ?? { x: node.x, y: node.y };
            const dim = neighbors !== null && !neighbors.has(node.entry.kc.id);
            const title = content.title(node.entry, locale);
            return (
              <g
                key={node.entry.kc.id}
                className={`graph-node graph-node-${node.status}${dim ? " dim" : ""}${selected === node.entry.kc.id ? " selected" : ""}`}
                transform={`translate(${position.x} ${position.y})`}
                role="button"
                tabIndex={0}
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
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    setSelected(node.entry.kc.id);
                  }
                }}
              >
                <circle r="70" className="graph-hit" />
                <circle r="38" className="graph-node-disc" />
                <text textAnchor="middle" y="5" className="graph-node-label">
                  {title.length > 18 ? `${title.slice(0, 17)}…` : title}
                </text>
                <text textAnchor="middle" y="62" className="graph-node-status">
                  {t(`graph-status-${node.status}`)}
                </text>
              </g>
            );
          })}
        </svg>
        {nodes.length === 0 ? <p className="bento-empty">{t("graph-empty")}</p> : null}
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
