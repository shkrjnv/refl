import { describe, expect, it } from "vitest";

import { ACTIVITY_DAYS, activitySummary, localDay } from "./activity";

describe("activity heatmap", () => {
  it("counts unique answer events by local day", () => {
    const now = new Date(2026, 10, 27, 12).getTime();
    const first = now - 2 * 86_400_000;
    const summary = activitySummary(
      [
        {
          event_id: "1",
          event: { client_ts: first, payload: { type: "answer" } },
        },
        {
          event_id: "1",
          event: { client_ts: first, payload: { type: "answer" } },
        },
        {
          event_id: "2",
          event: { client_ts: now, payload: { type: "answer" } },
        },
        {
          event_id: "3",
          event: { client_ts: now, payload: { type: "theme_selected" } },
        },
      ],
      null,
      now,
    );
    expect(summary.days).toHaveLength(ACTIVITY_DAYS);
    expect(summary.days[0]?.weekday).toBe(0);
    expect(summary.total).toBe(2);
    expect(summary.todayCount).toBe(1);
    expect(summary.bestStreak).toBe(1);
    expect(localDay(now)).toBe("2026-11-27");
  });

  it("uses the folded state's last-answer days for compacted history", () => {
    const now = new Date(2026, 10, 27, 12).getTime();
    const state = JSON.stringify({
      last_answer: {
        one: now,
        two: now - 86_400_000,
      },
    });
    const summary = activitySummary([], state, now);
    expect(summary.todayCount).toBe(1);
    expect(summary.bestStreak).toBe(2);
  });

  it("does not count placement answers as learning activity", () => {
    const now = new Date(2026, 10, 27, 12).getTime();
    const state = JSON.stringify({ last_answer: { en: now } });
    const summary = activitySummary(
      [
        {
          event_id: "placement",
          event: {
            client_ts: now,
            payload: { type: "answer", context: "placement" },
          },
        },
      ],
      state,
      now,
    );
    expect(summary.todayCount).toBe(0);
    expect(summary.total).toBe(0);
  });
});
