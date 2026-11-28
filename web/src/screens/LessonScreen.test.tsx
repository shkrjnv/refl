import "fake-indexeddb/auto";

import { afterEach, describe, expect, it } from "vitest";

import type { SyncRequest, SyncResponse } from "../api/client";
import { LearnerStorage } from "../db/storage";
import {
  LessonRunner,
  type LessonCore,
} from "../prototype/lesson";
import { SyncEngine, type SyncApi, type SyncCore } from "../sync/engine";
import { ClientClock } from "../sync/uuidv7";
import { isJsonObject, type JsonValue } from "../types/json";
import type {
  AnswerCheckResult,
  CoreInfo,
  FoldResult,
  NextItemResult,
  PlanSessionResult,
  ReplanSessionResult,
} from "../wasm/types";
import { testContent } from "./testContent";

const LEARNER = "lesson-test-learner";
const KC = "en.gram.article.a-an-sound";
const ITEM = { id: "test.gap", version: 1 };
const NOW = 1_700_000_000_000;

class FakeLessonCore implements LessonCore, SyncCore {
  #next = 0;
  readonly #plan = {
    steps: [{ phase: "block" }, { phase: "block" }],
    done: [],
    learn: [{ kc: KC, lesson: true }],
  };

  async coreInfo(): Promise<CoreInfo> {
    return {
      core_version: "0.0.0",
      schema_version: 2,
      fold_version: "fold-inputs-2",
      max_bundle_format: 10,
    };
  }

  async planSession(): Promise<PlanSessionResult> {
    return { plan: this.#plan, issued: {} };
  }

  async nextItem(): Promise<NextItemResult> {
    this.#next += 1;
    if (this.#next === 1) {
      return {
        kind: "lesson",
        kc: KC,
        explanation: "test.explanation",
      };
    }
    if (this.#next === 2) {
      return {
        kind: "item",
        item: ITEM,
        slot: 0,
        issued: {},
        context: "practice",
      };
    }
    return { kind: "finished", end: "nothing_due" };
  }

  async replanSession(): Promise<ReplanSessionResult> {
    return { plan: this.#plan, rule: null, issued: [] };
  }

  async checkAnswer(
    _bundle: string,
    _lexicon: string,
    item: JsonValue,
  ): Promise<AnswerCheckResult> {
    return {
      kind: "checked",
      item,
      slot: 0,
      verdict: "correct",
      misconception: null,
      elapsed_ms: 12,
    };
  }

  async foldState(
    _bundle: string,
    _lexicon: string,
    _snapshot: string | null,
    events: JsonValue[],
  ): Promise<FoldResult> {
    const last = events.at(-1);
    let lastEventId = "";
    if (isJsonObject(last)) {
      const id = last["event_id"];
      if (typeof id === "string") {
        lastEventId = id;
      }
    }
    return {
      snapshot: { kcs: {} },
      state_hash: "test-hash",
      incomplete: false,
      last_event_id: lastEventId,
    };
  }

  async decodeEvent(event: JsonValue) {
    return { kind: "event" as const, event };
  }
}

class FakeSyncApi implements SyncApi {
  requests: SyncRequest[] = [];

  async sync(request: SyncRequest): Promise<SyncResponse> {
    this.requests.push(request);
    const accepted = request.events.flatMap((event) => {
      if (!isJsonObject(event)) {
        return [];
      }
      const id = event["event_id"];
      return typeof id === "string" ? [id] : [];
    });
    return {
      accepted,
      rejected: [],
      events: [],
      cursor: 0,
      more: false,
      server_time: NOW,
      min_core_version: "0.0.0",
      latest_content_version: "test-content",
    };
  }
}

const opened: LearnerStorage[] = [];

async function storage(): Promise<LearnerStorage> {
  const store = await LearnerStorage.open(LEARNER);
  opened.push(store);
  return store;
}

afterEach(async () => {
  for (const store of opened.splice(0)) {
    await store.wipe().catch(() => undefined);
  }
});

describe("offline lesson flow", () => {
  it("runs pretest, explanation, practice, summary and flushes the outbox", async () => {
    const store = await storage();
    const core = new FakeLessonCore();
    const clock = new ClientClock(0, () => NOW);
    const content = testContent();
    const runner = new LessonRunner({
      storage: store,
      core,
      content,
      learnerId: LEARNER,
      locale: "en",
      clock,
    });

    let view = await runner.start();
    expect(view.phase).toBe("pretest");
    expect(view.item?.stimulus).toContain("I have");
    expect(await store.listOutbox()).toHaveLength(0);

    view = await runner.submitPretest({ kind: "text", value: "a" });
    expect(view.phase).toBe("explanation");
    expect(view.feedback?.correct).toBe(true);
    expect(await store.listOutbox()).toHaveLength(1);

    view = await runner.continueExplanation();
    expect(view.phase).toBe("practice");
    expect(view.item?.ref.id).toBe(ITEM.id);

    view = await runner.submitAnswer({ kind: "text", value: "a" });
    expect(view.phase).toBe("feedback");
    expect(await store.listOutbox()).toHaveLength(2);

    view = await runner.advanceAfterFeedback();
    expect(view.phase).toBe("summary");
    expect(view.summary?.answered).toBe(2);
    expect(view.summary?.accuracy).toBe(100);
    expect(await store.getSnapshot()).not.toBeNull();

    const outbox = await store.listOutbox();
    for (const record of outbox) {
      expect(isJsonObject(record.event)).toBe(true);
      const payload = isJsonObject(record.event)
        ? record.event["payload"]
        : null;
      expect(isJsonObject(payload) ? payload["type"] : null).toBe("answer");
    }

    const api = new FakeSyncApi();
    const engine = new SyncEngine({
      storage: store,
      api,
      core,
      coreVersion: "0.0.0",
      contentVersion: content.contentVersion,
      clock: new ClientClock(0, () => NOW),
    });
    const result = await engine.run();

    expect(result.accepted).toHaveLength(2);
    expect(api.requests).toHaveLength(1);
    expect(api.requests[0]?.events).toHaveLength(2);
    expect(await store.listPendingOutbox()).toHaveLength(0);
  });
});
