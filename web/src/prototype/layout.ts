import { isJsonObject } from "../types/json";

export type NavigationPosition = "left" | "right" | "bottom";

export interface DashboardLayout {
  nav: NavigationPosition;
}

export const LAYOUT_KEY = "refl.layout.v1";

export function defaultLayout(): DashboardLayout {
  return { nav: "left" };
}

function isNavigationPosition(value: unknown): value is NavigationPosition {
  return value === "left" || value === "right" || value === "bottom";
}

export function normalizeLayout(value: unknown): DashboardLayout {
  if (!isJsonObject(value)) {
    return defaultLayout();
  }
  return {
    nav: isNavigationPosition(value["nav"]) ? value["nav"] : "left",
  };
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
