import { describe, expect, it } from "vitest";

import { activitySummary } from "./activity";
import { retrievability } from "./progress";

describe("FSRS telemetry", () => {
  it("returns 0.9 retrievability when elapsed time equals stability", () => {
    const now = 1_700_000_000_000;
    expect(retrievability(10, now - 10 * 86_400_000, now)).toBeCloseTo(0.9, 5);
    expect(retrievability(10, now, now)).toBe(1);
    expect(retrievability(0, now - 86_400_000, now)).toBe(0);
  });

  it("keeps the seven-week activity window aligned to whole weeks", () => {
    const now = new Date(2026, 10, 27, 12).getTime();
    const summary = activitySummary([], null, now);
    expect(summary.days).toHaveLength(49);
    expect(summary.days[0]?.weekday).toBe(0);
    expect(summary.days[6]?.weekday).toBe(6);
  });
});
