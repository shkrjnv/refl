import { describe, expect, it } from "vitest";

import { ACTIVITY_DAYS, activitySummary } from "./activity";
import {
  levelView,
  retrievability,
  type KcProgress,
} from "./progress";

function kc(
  level: string,
  percent: number,
  observations = 1,
  mastered = false,
): KcProgress {
  return {
    kcId: `${level}.${percent}`,
    title: level,
    mastered,
    observations,
    stabilityDays: 0,
    difficulty: 0,
    percent,
    lastReview: null,
    due: false,
    level,
    domain: "grammar",
  };
}

describe("FSRS telemetry", () => {
  it("returns 0.9 retrievability when elapsed time equals stability", () => {
    const now = 1_700_000_000_000;
    expect(retrievability(10, now - 10 * 86_400_000, now)).toBeCloseTo(0.9, 5);
    expect(retrievability(10, now - 90 * 86_400_000, now)).toBeCloseTo(0.5, 5);
    expect(retrievability(10, now, now)).toBe(1);
    expect(retrievability(0, now - 86_400_000, now)).toBe(0);
  });

  it("keeps the recent activity window aligned to whole weeks", () => {
    const now = new Date(2026, 10, 27, 12).getTime();
    const summary = activitySummary([], null, now);
    expect(summary.days).toHaveLength(ACTIVITY_DAYS);
    expect(summary.days[0]?.weekday).toBe(0);
    expect(summary.days[6]?.weekday).toBe(6);
  });
});

describe("honest level view", () => {
  it("shows Pre-A1 when there is no evidence and no placement", () => {
    const view = levelView([], null);
    expect(view).toMatchObject({
      current: "Pre-A1",
      next: "A1",
      percent: 0,
      total: 0,
      preA1: true,
    });
  });

  it("uses a continuous placement position and the current level's rules", () => {
    const progress = [kc("A1", 50), kc("A1", 50)];
    const view = levelView(progress, 1.4);
    expect(view).toMatchObject({
      current: "A1",
      next: "A2",
      percent: 50,
      mastered: 0,
      total: 2,
      preA1: false,
    });
  });

  it("shows a fractional pre-A1 position when placement landed below A1", () => {
    const view = levelView([], 0.8);
    expect(view).toMatchObject({
      current: "Pre-A1",
      next: "A1",
      percent: 80,
      total: 0,
      preA1: true,
    });
  });

  it("clamps a stored level above the scale", () => {
    const view = levelView([], 9);
    expect(view.current).toBe("C2");
    expect(view.next).toBe("C2");
  });

  it("keeps the curriculum view for a learner with answers but no placement", () => {
    const view = levelView([kc("A1", 40)], null);
    expect(view).toMatchObject({
      current: "A1",
      next: "A2",
      percent: 40,
      total: 1,
      preA1: false,
    });
  });
});
