import { isJsonObject } from "../types/json";

export const widgetIds = [
  "hero",
  "rules",
  "memory",
  "misconceptions",
  "activity",
] as const;

export type WidgetId = (typeof widgetIds)[number];
export const widgetColSpans = [1, 2, 3] as const;
export type WidgetColSpan = (typeof widgetColSpans)[number];
export const widgetRowSpans = ["compact", "tall"] as const;
export type WidgetRowSpan = (typeof widgetRowSpans)[number];
export type NavigationPosition = "left" | "right" | "bottom";

export const widgetMinColSpans: Record<WidgetId, WidgetColSpan> = {
  hero: 2,
  rules: 2,
  memory: 1,
  misconceptions: 1,
  activity: 1,
};

export interface DashboardLayout {
  nav: NavigationPosition;
  order: WidgetId[];
  colSpan: Record<WidgetId, WidgetColSpan>;
  rowSpan: Record<WidgetId, WidgetRowSpan>;
}

export const LAYOUT_KEY = "refl.layout.v1";

export const defaultWidgetColSpans: Record<WidgetId, WidgetColSpan> = {
  hero: 3,
  rules: 3,
  memory: 1,
  misconceptions: 1,
  activity: 1,
};

export const defaultWidgetRowSpans: Record<WidgetId, WidgetRowSpan> = {
  hero: "tall",
  rules: "compact",
  memory: "compact",
  misconceptions: "compact",
  activity: "compact",
};

export function defaultLayout(): DashboardLayout {
  return {
    nav: "left",
    order: [...widgetIds],
    colSpan: { ...defaultWidgetColSpans },
    rowSpan: { ...defaultWidgetRowSpans },
  };
}

function isWidgetId(value: unknown): value is WidgetId {
  return typeof value === "string" && widgetIds.includes(value as WidgetId);
}

function isNavigationPosition(value: unknown): value is NavigationPosition {
  return value === "left" || value === "right" || value === "bottom";
}

function isWidgetColSpan(value: unknown): value is WidgetColSpan {
  return value === 1 || value === 2 || value === 3;
}

function isWidgetRowSpan(value: unknown): value is WidgetRowSpan {
  return value === "compact" || value === "tall";
}

export function normalizeLayout(value: unknown): DashboardLayout {
  const base = defaultLayout();
  if (!isJsonObject(value)) {
    return base;
  }
  const nav = isNavigationPosition(value["nav"]) ? value["nav"] : base.nav;
  const order: WidgetId[] = [];
  if (Array.isArray(value["order"])) {
    for (const candidate of value["order"]) {
      if (isWidgetId(candidate) && !order.includes(candidate)) {
        order.push(candidate);
      }
    }
  }
  for (const id of widgetIds) {
    if (!order.includes(id)) {
      order.push(id);
    }
  }
  const colSpan = { ...base.colSpan };
  const storedColSpans = value["colSpan"];
  if (isJsonObject(storedColSpans)) {
    for (const id of widgetIds) {
      const span = storedColSpans[id];
      if (isWidgetColSpan(span)) {
        colSpan[id] = span;
      }
    }
  } else if (isJsonObject(value["sizes"])) {
    for (const id of widgetIds) {
      const legacy = value["sizes"][id];
      if (legacy === "wide") {
        colSpan[id] = 3;
      } else if (legacy === "compact") {
        colSpan[id] = 1;
      }
    }
  }
  const rowSpan = { ...base.rowSpan };
  const storedRowSpans = value["rowSpan"];
  if (isJsonObject(storedRowSpans)) {
    for (const id of widgetIds) {
      const span = storedRowSpans[id];
      if (isWidgetRowSpan(span)) {
        rowSpan[id] = span;
      }
    }
  }
  for (const id of widgetIds) {
    colSpan[id] = Math.max(widgetMinColSpans[id], colSpan[id]) as WidgetColSpan;
  }
  return { nav, order, colSpan, rowSpan };
}

export function readLayout(): DashboardLayout {
  if (typeof window === "undefined") {
    return defaultLayout();
  }
  try {
    const raw = window.localStorage.getItem(LAYOUT_KEY);
    if (!raw) {
      return defaultLayout();
    }
    return normalizeLayout(JSON.parse(raw) as unknown);
  } catch {
    return defaultLayout();
  }
}

export function storeLayout(layout: DashboardLayout): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(
      LAYOUT_KEY,
      JSON.stringify(normalizeLayout(layout)),
    );
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

export function resetLayout(): DashboardLayout {
  const next = defaultLayout();
  storeLayout(next);
  return next;
}

export function moveWidget(
  order: readonly WidgetId[],
  id: WidgetId,
  direction: -1 | 1,
): WidgetId[] {
  const next = [...order];
  const index = next.indexOf(id);
  const target = index + direction;
  if (index < 0 || target < 0 || target >= next.length) {
    return next;
  }
  const [item] = next.splice(index, 1);
  if (item) {
    next.splice(target, 0, item);
  }
  return next;
}

export function reorderWidget(
  order: readonly WidgetId[],
  source: WidgetId,
  target: WidgetId,
): WidgetId[] {
  if (source === target) {
    return [...order];
  }
  const next = order.filter((id) => id !== source);
  const targetIndex = next.indexOf(target);
  if (targetIndex < 0) {
    return [...order];
  }
  next.splice(targetIndex, 0, source);
  return next;
}

export function updateWidgetSpan(
  layout: DashboardLayout,
  id: WidgetId,
  span: { colSpan?: WidgetColSpan; rowSpan?: WidgetRowSpan },
): DashboardLayout {
  const requested = span.colSpan ?? layout.colSpan[id];
  return {
    ...layout,
    colSpan: {
      ...layout.colSpan,
      [id]: Math.max(widgetMinColSpans[id], requested) as WidgetColSpan,
    },
    rowSpan: {
      ...layout.rowSpan,
      [id]: span.rowSpan ?? layout.rowSpan[id],
    },
  };
}

export function cycleWidgetColSpan(current: WidgetColSpan): WidgetColSpan {
  return current === 1 ? 2 : current === 2 ? 3 : 1;
}

export function toggleWidgetRowSpan(current: WidgetRowSpan): WidgetRowSpan {
  return current === "compact" ? "tall" : "compact";
}
