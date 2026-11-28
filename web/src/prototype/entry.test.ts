// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import type { LearnerStorage } from "../db/storage";
import { ClientClock } from "../sync/uuidv7";
import type { PrototypeContent } from "./content";
import {
  completeEntryChoice,
  entryDecision,
  hasLearningHistory,
  readEntryChoice,
  recordZeroEntry,
  resetEntryChoice,
} from "./entry";

const learner = "01930000-0000-7000-8000-0000000000aa";

afterEach(() => {
  window.localStorage.clear();
});

describe("entry decision", () => {
  it("needs the gate only for a learner with no choice, profile or history", () => {
    expect(entryDecision(null, learner)).toBe("needed");
    expect(entryDecision(null, learner, true)).toBe("done");
  });

  it("treats a zero start level as a completed entry", () => {
    const state = JSON.stringify({
      profile: { start_level: { en: 0 } },
    });
    expect(entryDecision(state, learner)).toBe("done");
  });

  it("keeps legacy users out of the gate", () => {
    expect(
      entryDecision(JSON.stringify({ kcs: { a: {} } }), learner),
    ).toBe("done");
    expect(
      entryDecision(
        JSON.stringify({ misconceptions: { a: {} } }),
        learner,
      ),
    ).toBe("done");
    expect(
      entryDecision(JSON.stringify({ last_answer: { en: 1 } }), learner),
    ).toBe("done");
  });

  it("counts only learning events as legacy evidence", () => {
    expect(
      hasLearningHistory([
        { event: { payload: { type: "theme_selected" } } },
      ]),
    ).toBe(false);
    expect(
      hasLearningHistory([{ event: { payload: { type: "answer" } } }]),
    ).toBe(true);
    expect(
      hasLearningHistory([
        { event: { payload: { type: "placement_completed" } } },
      ]),
    ).toBe(true);
  });

  it("keeps the gate for an interrupted placement even with placement answers", () => {    window.localStorage.setItem(
      "refl.placement-progress",
      JSON.stringify({
        learner_id: learner,
        placement: { subject: "en", asked: [], correct: 0 },
      }),
    );
    expect(
      entryDecision(JSON.stringify({ kcs: { a: {} } }), learner),
    ).toBe("needed");
  });

  it("is per learner and survives a local choice", () => {
    completeEntryChoice("learner-a", "start_from_zero");
    expect(readEntryChoice("learner-a")).toBe("start_from_zero");
    expect(entryDecision(null, "learner-a")).toBe("done");
    expect(entryDecision(null, "learner-b")).toBe("needed");
    resetEntryChoice("learner-b");
    expect(readEntryChoice("learner-a")).toBe("start_from_zero");
    resetEntryChoice("learner-a");
    expect(readEntryChoice("learner-a")).toBeNull();
  });
});

describe("zero entry event", () => {
  it("records placement_completed with a zero start level", async () => {
    const outbox: { event_id: string; event: unknown }[] = [];
    const snapshots: unknown[] = [];
    const storage = {
      getClockOffset: async () => 0,
      queueOutbox: async (record: { event_id: string; event: unknown }) => {
        outbox.push(record);
      },
      putSnapshot: async (record: unknown) => {
        snapshots.push(record);
      },
    } as unknown as LearnerStorage;
    const core = {
      coreInfo: async () => ({
        core_version: "0.0.0",
        schema_version: 2,
        fold_version: "fold",
        max_bundle_format: 10,
      }),
      foldState: async () => ({
        snapshot: { profile: {} },
        state_hash: "hash",
        incomplete: false,
        last_event_id: "event",
      }),
    };
    const content = {
      bundleJson: "{}",
      lexiconJson: "",
      contentVersion: "content",
    } as unknown as PrototypeContent;
    await recordZeroEntry(
      {
        storage,
        core,
        content,
        learnerId: learner,
        clock: new ClientClock(),
      },
      null,
    );
    expect(outbox).toHaveLength(1);
    expect(snapshots).toHaveLength(1);
    const payload = (
      outbox[0]?.event as { payload?: Record<string, unknown> }
    ).payload;
    expect(payload).toMatchObject({
      type: "placement_completed",
      start_level: { en: 0 },
      answered: 0,
      source: "start_from_zero",
    });
    expect(readEntryChoice(learner)).toBe("start_from_zero");
  });
});
