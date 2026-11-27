// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import {
  LAYOUT_KEY,
  defaultLayout,
  defaultWidgetColSpans,
  defaultWidgetRowSpans,
  moveWidget,
  normalizeLayout,
  readLayout,
  reorderWidget,
  resetLayout,
  storeLayout,
  updateWidgetSpan,
  widgetIds,
  cycleWidgetColSpan,
  toggleWidgetRowSpan,
  type DashboardLayout,
} from "./layout";

afterEach(() => {
  window.localStorage.clear();
});

describe("dashboard layout persistence", () => {
  it("stores order, column spans, row spans, and navigation under refl.layout.v1", () => {
    const layout = defaultLayout();
    const next: DashboardLayout = {
      ...layout,
      nav: "bottom",
      order: ["activity", ...layout.order.filter((id) => id !== "activity")],
      colSpan: { ...layout.colSpan, activity: 2 },
      rowSpan: { ...layout.rowSpan, activity: "tall" },
    };
    storeLayout(next);
    const raw = window.localStorage.getItem(LAYOUT_KEY);
    expect(raw).not.toBeNull();
    expect(readLayout()).toEqual(next);
    expect(raw).toContain('"colSpan"');
    expect(raw).toContain('"rowSpan"');
  });

  it("repairs missing, duplicate, and unknown widgets", () => {
    const layout = normalizeLayout({
      nav: "sideways",
      order: ["activity", "activity", "unknown", "hero"],
      colSpan: { activity: 2, rules: 9 },
      rowSpan: { memory: "tall", rules: "huge" },
    });
    expect(layout.nav).toBe("left");
    expect(layout.order.slice(0, 2)).toEqual(["activity", "hero"]);
    expect(layout.order).toHaveLength(widgetIds.length);
    expect(new Set(layout.order).size).toBe(widgetIds.length);
    expect(layout.colSpan.activity).toBe(2);
    expect(layout.colSpan.rules).toBe(defaultWidgetColSpans.rules);
    expect(layout.rowSpan.memory).toBe("tall");
    expect(layout.rowSpan.rules).toBe(defaultWidgetRowSpans.rules);
  });

  it("migrates the previous sizes field to three-column spans", () => {
    const layout = normalizeLayout({
      order: ["activity", "memory", "hero", "rules", "misconceptions"],
      sizes: {
        activity: "wide",
        memory: "compact",
        rules: "wide",
      },
    });
    expect(layout.colSpan.activity).toBe(3);
    expect(layout.colSpan.memory).toBe(1);
    expect(layout.colSpan.rules).toBe(3);
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

  it("updates spans, cycles controls, and resets the stored layout", () => {
    const layout = defaultLayout();
    const resized = updateWidgetSpan(layout, "memory", {
      colSpan: 2,
      rowSpan: "tall",
    });
    expect(resized.colSpan.memory).toBe(2);
    expect(resized.rowSpan.memory).toBe("tall");
    expect(cycleWidgetColSpan(1)).toBe(2);
    expect(cycleWidgetColSpan(2)).toBe(3);
    expect(cycleWidgetColSpan(3)).toBe(1);
    expect(toggleWidgetRowSpan("compact")).toBe("tall");
    storeLayout({
      ...resized,
      nav: "right",
      order: ["activity", "memory", "hero", "rules", "misconceptions"],
    });
    expect(readLayout().nav).toBe("right");
    expect(resetLayout()).toEqual(defaultLayout());
    expect(readLayout()).toEqual(defaultLayout());
    expect(window.localStorage.getItem(LAYOUT_KEY)).toContain('"nav":"left"');
  });
});
