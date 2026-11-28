// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";

import type { LearnerStorage } from "../db/storage";
import { testContent } from "../screens/testContent";
import { ClientClock } from "../sync/uuidv7";
import type { PlacementCore } from "./placement";
import { PlacementRunner } from "./placement";
import { readEntryChoice, resetEntryChoice } from "./entry";

const learner = "01930000-0000-7000-8000-0000000000aa";
const ref = { id: "test.gap", version: 1 };
const placement = { subject: "en", asked: [], correct: 0 };

function fakeStorage(events: { event: unknown }[]) {
  return {
    getClockOffset: async () => 0,
    queueOutbox: async (record: { event: unknown }) => {
      events.push(record);
    },
    putSnapshot: async () => undefined,
  } as unknown as LearnerStorage;
}

function fakeCore(overrides: Partial<PlacementCore> = {}) {
  const core: PlacementCore = {
    coreInfo: async () => ({
      core_version: "0.0.0",
      schema_version: 2,
      fold_version: "fold",
      max_bundle_format: 10,
    }),
    foldState: async () => ({
      snapshot: {},
      state_hash: "hash",
      incomplete: false,
      last_event_id: "event",
    }),
    checkAnswer: vi.fn(async () => ({
      kind: "checked" as const,
      item: ref,
      slot: 0,
      verdict: "correct" as const,
      misconception: null,
      elapsed_ms: 10,
    })),
    placementStart: vi.fn(async () => ({
      kind: "item" as const,
      item: ref,
      slot: 0,
      placement,
    })),
    placementNext: vi.fn(async () => ({
      kind: "item" as const,
      item: ref,
      slot: 0,
      placement,
    })),
    placementAnswer: vi.fn(async () => ({
      kind: "done" as const,
      placement: { subject: "en", asked: [{ item: ref, slot: 0 }], correct: 1 },
      level: { point: 1, low: 0, high: 1.5 },
      start_level: 1,
      answered: 1,
      source: "diagnostic" as const,
    })),
    ...overrides,
  };
  return core;
}

afterEach(() => {
  window.localStorage.clear();
  resetEntryChoice();
});

describe("placement runner", () => {
  it("locks updates for the lifetime of the check", async () => {
    let active = false;
    const runner = new PlacementRunner({
      storage: fakeStorage([]),
      core: fakeCore(),
      content: testContent(),
      learnerId: learner,
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
    await runner.start(null);
    expect(active).toBe(true);
    runner.finish();
    expect(active).toBe(false);
  });

  it("logs a placement answer and the diagnostic completion event", async () => {
    const outbox: { event: unknown }[] = [];
    const core = fakeCore();
    const runner = new PlacementRunner({
      storage: fakeStorage(outbox),
      core,
      content: testContent(),
      learnerId: learner,
      locale: "en",
      clock: new ClientClock(),
    });
    const started = await runner.start(null);
    expect(started.phase).toBe("item");
    const done = await runner.submit({ kind: "text", value: "a" });
    expect(done.phase).toBe("result");
    if (done.phase === "result") {
      expect(done.result.ceiling).toBe(true);
    }
    expect(core.checkAnswer).toHaveBeenCalledTimes(1);
    expect(core.placementAnswer).toHaveBeenCalledTimes(1);

    const payloads = outbox.map(
      (record) =>
        (record.event as { payload: Record<string, unknown> }).payload,
    );
    expect(payloads[0]).toMatchObject({
      type: "answer",
      context: "placement",
    });
    expect(payloads[1]).toMatchObject({
      type: "placement_completed",
      start_level: { en: 1 },
      answered: 1,
      source: "diagnostic",
    });
    expect(readEntryChoice(learner)).toBe("diagnostic");
  });

  it("resumes from the saved placement progress", async () => {
    const core = fakeCore();
    const runner = new PlacementRunner({
      storage: fakeStorage([]),
      core,
      content: testContent(),
      learnerId: learner,
      locale: "en",
      clock: new ClientClock(),
    });
    await runner.start(null);
    const resumed = new PlacementRunner({
      storage: fakeStorage([]),
      core,
      content: testContent(),
      learnerId: learner,
      locale: "en",
      clock: new ClientClock(),
    });
    const view = await resumed.start(null);
    expect(view.phase).toBe("item");
    expect(core.placementNext).toHaveBeenCalledTimes(1);
  });

  it("surfaces an unavailable pool without inventing a level", async () => {
    const core = fakeCore({
      placementStart: vi.fn(async () => ({
        kind: "unavailable" as const,
        placement,
      })),
    });
    const runner = new PlacementRunner({
      storage: fakeStorage([]),
      core,
      content: testContent(),
      learnerId: learner,
      locale: "en",
      clock: new ClientClock(),
    });
    const view = await runner.start(null);
    expect(view.phase).toBe("unavailable");
  });
});
