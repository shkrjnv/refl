import { describe, expect, it, vi } from "vitest";

import type { LearnerStorage } from "../db/storage";
import { testContent } from "../screens/testContent";
import { ClientClock } from "../sync/uuidv7";
import type { LessonCore } from "./lesson";
import { LessonRunner } from "./lesson";

function storage(): LearnerStorage {
  return {
    getClockOffset: async () => 0,
    getSnapshot: async () => null,
    putSnapshot: async () => undefined,
    queueOutbox: async () => undefined,
  } as unknown as LearnerStorage;
}

function core(overrides: Partial<LessonCore> = {}): LessonCore {
  return {
    coreInfo: async () => ({
      core_version: "0.0.0",
      schema_version: 2,
      fold_version: "fold",
      max_bundle_format: 10,
    }),
    planSession: async () => ({ plan: {}, issued: null }),
    nextItem: async () => ({ kind: "finished", end: {} }),
    replanSession: async () => ({ plan: {}, rule: null, issued: null }),
    checkAnswer: vi.fn(),
    foldState: async () => ({
      snapshot: {},
      state_hash: "hash",
      incomplete: false,
      last_event_id: null,
    }),
    ...overrides,
  } as LessonCore;
}

describe("lesson runner", () => {
  it("locks updates until the lesson screen ends", async () => {
    let active = false;
    const runner = new LessonRunner({
      storage: storage(),
      core: core(),
      content: testContent(),
      learnerId: "01930000-0000-7000-8000-0000000000aa",
      locale: "en",
      clock: new ClientClock(),
      gate: {
        beginSession: () => {
          active = true;
        },
        endSession: () => {
          active = false;
        },
      },
    });
    const view = await runner.start();
    expect(view.phase).toBe("summary");
    expect(active).toBe(true);
    runner.finish();
    expect(active).toBe(false);
  });

  it("releases the lock when planning fails", async () => {
    let active = false;
    const runner = new LessonRunner({
      storage: storage(),
      core: core({
        planSession: async () => {
          throw new Error("no plan");
        },
      }),
      content: testContent(),
      learnerId: "01930000-0000-7000-8000-0000000000aa",
      locale: "en",
      clock: new ClientClock(),
      gate: {
        beginSession: () => {
          active = true;
        },
        endSession: () => {
          active = false;
        },
      },
    });
    await expect(runner.start()).rejects.toThrow("no plan");
    expect(active).toBe(false);
  });
});
