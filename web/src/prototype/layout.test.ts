// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import {
  LAYOUT_KEY,
  defaultLayout,
  type DashboardLayout,
  moveWidget,
  normalizeLayout,
  readLayout,
  reorderWidget,
  resetLayout,
  storeLayout,
  toggleWidgetSize,
  widgetIds,
} from "./layout";

afterEach(() => {
  window.localStorage.clear();
});

describe("dashboard layout persistence", () => {
  it("stores widget order, sizes, and navigation position under refl.layout.v1", () => {
    const layout = defaultLayout();
    const next: DashboardLayout = {
      ...layout,
      nav: "bottom" as const,
      order: [
        "activity",
        ...layout.order.filter((id) => id !== "activity"),
      ],
      sizes: { ...layout.sizes, activity: "wide" as const },
    };
    storeLayout(next);
    const raw = window.localStorage.getItem(LAYOUT_KEY);
    expect(raw).not.toBeNull();
    expect(readLayout()).toEqual(next);
  });

  it("repairs missing, duplicate, and unknown widgets", () => {
    const layout = normalizeLayout({
      nav: "sideways",
      order: ["activity", "activity", "unknown", "hero"],
      sizes: { activity: "huge", rules: "compact" },
    });
    expect(layout.nav).toBe("left");
    expect(layout.order.slice(0, 2)).toEqual(["activity", "hero"]);
    expect(layout.order).toHaveLength(widgetIds.length);
    expect(new Set(layout.order).size).toBe(widgetIds.length);
    expect(layout.sizes.activity).toBe("compact");
    expect(layout.sizes.rules).toBe("compact");
  });

  it("reorders and moves widgets without dropping any card", () => {
    const order = defaultLayout().order;
    expect(moveWidget(order, "rules", -1)).toEqual([
      "rules",
      "hero",
      "memory",
      "misconceptions",
      "activity",
    ]);
    expect(reorderWidget(order, "activity", "hero")).toEqual([
      "activity",
      "hero",
      "rules",
      "memory",
      "misconceptions",
    ]);
    expect(new Set(reorderWidget(order, "memory", "activity"))).toEqual(
      new Set(widgetIds),
    );
  });

  it("toggles compact and wide sizes and resets both order and navigation", () => {
    const layout = defaultLayout();
    expect(toggleWidgetSize(layout.sizes, "memory").memory).toBe("wide");
    storeLayout({
      ...layout,
      nav: "right",
      order: ["activity", "memory", "hero", "rules", "misconceptions"],
      sizes: { ...layout.sizes, memory: "wide" },
    });
    expect(readLayout().nav).toBe("right");
    const reset = resetLayout();
    expect(reset).toEqual(defaultLayout());
    expect(readLayout()).toEqual(defaultLayout());
    expect(window.localStorage.getItem(LAYOUT_KEY)).toContain('"nav":"left"');
  });
});
