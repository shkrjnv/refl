import { useLayoutEffect, useMemo, useRef, useState } from "react";

import {
  LOCAL_GRAPH_PILL,
  localGraphLayout,
  type LocalGraphEdge,
  type LocalGraphNode,
  type WikiKind,
} from "../prototype/notes";
import { StudioIcon, type StudioIconName } from "./StudioIcons";

const KIND_ICON: Record<WikiKind, StudioIconName> = {
  note: "note",
  kc: "graph",
  lexeme: "vocabulary",
  book: "library",
  highlight: "bookmark",
};

/** Inspector panels are about this wide before the first measurement. */
const DEFAULT_WIDTH = 280;

export interface NotesLocalGraphProps {
  nodes: readonly LocalGraphNode[];
  edges: readonly LocalGraphEdge[];
  /** Display label per node id (localized titles, lemmas, book names). */
  labels: ReadonlyMap<string, string>;
  centerId: string;
  ariaLabel: string;
  kindLabel: (kind: WikiKind) => string;
  onSelect: (node: LocalGraphNode) => void;
}

/**
 * Radial neighbourhood of the active note. Pill sizes and positions come
 * from `localGraphLayout`, so labels wrap at spaces and never overlap.
 */
export function NotesLocalGraph({
  nodes,
  edges,
  labels,
  centerId,
  ariaLabel,
  kindLabel,
  onSelect,
}: NotesLocalGraphProps) {
  const ref = useRef<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(0);

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) {
      return;
    }
    const update = () => {
      const next = Math.round(element.clientWidth);
      setWidth((current) => (current === next ? current : next));
    };
    update();
    const observer =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(element);
    return () => observer?.disconnect();
  }, []);

  const placement = useMemo(
    () =>
      localGraphLayout(
        { nodes, edges },
        labels,
        width > 0 ? width : DEFAULT_WIDTH,
      ),
    [nodes, edges, labels, width],
  );

  return (
    <div
      ref={ref}
      className="notes-local-graph"
      role="group"
      aria-label={ariaLabel}
      style={{ height: `${placement.height}px` }}
    >
      <svg
        width={placement.width}
        height={placement.height}
        viewBox={`0 0 ${placement.width} ${placement.height}`}
        aria-hidden="true"
      >
        {placement.edges.map((edge) => (
          <line
            key={`${edge.from}-${edge.to}`}
            x1={edge.start.x}
            y1={edge.start.y}
            x2={edge.end.x}
            y2={edge.end.y}
          />
        ))}
      </svg>
      {nodes.map((node) => {
        const box = placement.boxes.get(node.id);
        if (!box) {
          return null;
        }
        const lines = placement.lines.get(node.id) ?? [labels.get(node.id) ?? node.label];
        const center = node.id === centerId;
        return (
          <button
            key={node.id}
            type="button"
            className={`notes-graph-node notes-graph-${node.kind}${center ? " center" : ""}`}
            style={{
              left: `${box.x}px`,
              top: `${box.y}px`,
              width: `${box.width}px`,
              height: `${box.height}px`,
              padding: `${LOCAL_GRAPH_PILL.padY}px ${LOCAL_GRAPH_PILL.padX}px`,
            }}
            aria-current={center ? "true" : undefined}
            aria-label={`${kindLabel(node.kind)}: ${lines.join(" ")}`}
            disabled={center}
            onClick={() => onSelect(node)}
          >
            <StudioIcon name={KIND_ICON[node.kind]} size={LOCAL_GRAPH_PILL.icon} />
            <span>
              {lines.map((line, index) => (
                <span key={index} className="notes-graph-line">
                  {line}
                </span>
              ))}
            </span>
          </button>
        );
      })}
    </div>
  );
}
