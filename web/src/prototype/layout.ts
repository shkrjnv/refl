import { isJsonObject } from "../types/json";

export const widgetIds = [
  "hero",
  "rules",
  "memory",
  "misconceptions",
  "activity",
] as const;

export type WidgetId = (typeof widgetIds)[number];
export type WidgetSize = "compact" | "wide";
export type NavigationPosition = "left" | "right" | "bottom";

export interface DashboardLayout {
  nav: NavigationPosition;
  order: WidgetId[];
  sizes: Record<WidgetId, WidgetSize>;
}

export const LAYOUT_KEY = "refl.layout.v1";

export const defaultWidgetSizes: Record<WidgetId, WidgetSize> = {
  hero: "wide",
  rules: "wide",
  memory: "compact",
  misconceptions: "compact",
  activity: "compact",
};

export function defaultLayout(): DashboardLayout {
  return {
    nav: "left",
    order: [...widgetIds],
    sizes: { ...defaultWidgetSizes },
  };
}

function isWidgetId(value: unknown): value is WidgetId {
  return (
    typeof value === "string" &&
    widgetIds.includes(value as WidgetId)
  );
}

function isNavigationPosition(value: unknown): value is NavigationPosition {
  return value === "left" || value === "right" || value === "bottom";
}

function isWidgetSize(value: unknown): value is WidgetSize {
  return value === "compact" || value === "wide";
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
  const sizes = { ...base.sizes };
  const storedSizes = value["sizes"];
  if (isJsonObject(storedSizes)) {
    for (const id of widgetIds) {
      const size = storedSizes[id];
      if (isWidgetSize(size)) {
        sizes[id] = size;
      }
    }
  }
  return { nav, order, sizes };
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
    window.localStorage.setItem(LAYOUT_KEY, JSON.stringify(normalizeLayout(layout)));
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

export function toggleWidgetSize(
  sizes: Record<WidgetId, WidgetSize>,
  id: WidgetId,
): Record<WidgetId, WidgetSize> {
  return {
    ...sizes,
    [id]: sizes[id] === "wide" ? "compact" : "wide",
  };
}
