import "fake-indexeddb/auto";

import { afterEach, describe, expect, it } from "vitest";

import type {
  ErasedResponse,
  SyncRequest,
  SyncResponse,
} from "../api/client";
import {
  LearnerStorage,
  sha256Hex,
  type ContentCandidate,
  type EventRecord,
} from "../db/storage";
import { isJsonObject, type JsonObject, type JsonValue } from "../types/json";
import { SessionUpdateGate } from "../worker/client";
import type { CoreInfo, DecodedEvent, FoldResult } from "../wasm/types";
import {
  MAX_SYNC_BATCH,
  SyncEngine,
  type SyncApi,
  type SyncCore,
} from "./engine";
import { ClientClock, createUuidV7Generator } from "./uuidv7";

const opened: LearnerStorage[] = [];
const LOCAL_TIME = 1_700_000_000_000;

async function openStorage(learnerId: string): Promise<LearnerStorage> {
  const storage = await LearnerStorage.open(learnerId);
  opened.push(storage);
  return storage;
}

function ids(count: number): string[] {
  const generator = createUuidV7Generator(
    () => LOCAL_TIME,
    () => new Uint8Array(8),
  );
  return Array.from({ length: count }, () => generator.next());
}

function event(learnerId: string, eventId: string): EventRecord {
  const value: JsonObject = {
    schema_version: 1,
    event_id: eventId,
    learner_id: learnerId,
    session_id: null,
    client_ts: LOCAL_TIME,
    server_ts: null,
    payload: { type: "lesson_started", lesson: "lesson" },
  };
  return {
    learner_id: learnerId,
    event_id: eventId,
    event: value,
    confirmed: false,
  };
}

function eventIdOf(value: JsonValue): string {
  if (!isJsonObject(value)) {
    throw new Error("event is not an object");
  }
  const eventId = value["event_id"];
  if (typeof eventId !== "string") {
    throw new Error("event id is missing");
  }
  return eventId;
}

function response(
  request: SyncRequest,
  overrides: Partial<SyncResponse> = {},
): SyncResponse {
  return {
    accepted: request.events.map(eventIdOf),
    rejected: [],
    events: [],
    cursor: 0,
    more: false,
    server_time: LOCAL_TIME,
    min_core_version: "0.0.0",
    latest_content_version: null,
    ...overrides,
  };
}

class ScriptedApi implements SyncApi {
  readonly requests: SyncRequest[] = [];
  readonly #respond: (
    request: SyncRequest,
    index: number,
  ) => SyncResponse | ErasedResponse;

  constructor(
    respond: (
      request: SyncRequest,
      index: number,
    ) => SyncResponse | ErasedResponse,
  ) {
    this.#respond = respond;
  }

  async sync(request: SyncRequest): Promise<SyncResponse | ErasedResponse> {
    const index = this.requests.length;
    this.requests.push(request);
    return this.#respond(request, index);
  }
}

class FakeCore implements SyncCore {
  readonly foldCalls: Array<{
    snapshotJson: string | null;
    eventCount: number;
  }> = [];

  async coreInfo(): Promise<CoreInfo> {
    return {
      core_version: "0.0.0",
      schema_version: 1,
      fold_version: "fold-inputs-1",
      max_bundle_format: 10,
    };
  }

  async decodeEvent(value: JsonValue): Promise<DecodedEvent> {
    return { kind: "event", event: value };
  }

  async foldState(
    _bundleJson: string,
    _lexiconJson: string,
    snapshotJson: string | null,
    events: JsonValue[],
  ): Promise<FoldResult> {
    this.foldCalls.push({ snapshotJson, eventCount: events.length });
    const last = events.at(-1);
    return {
      snapshot: { event_count: events.length },
      state_hash: "hash",
      incomplete: false,
      last_event_id: last ? eventIdOf(last) : "",
    };
  }
}

afterEach(async () => {
  for (const storage of opened.splice(0)) {
    await storage.wipe().catch(() => undefined);
  }
});

describe("sync engine", () => {
  it("sends ascending outbox batches of at most 500", async () => {
    const learnerId = "batch-learner";
    const storage = await openStorage(learnerId);
    const eventIds = ids(1200);
    for (const eventId of eventIds) {
      await storage.queueOutbox(event(learnerId, eventId));
    }
    const api = new ScriptedApi((request) => response(request));
    const engine = new SyncEngine({
      storage,
      api,
      core: new FakeCore(),
      coreVersion: "0.0.0",
      contentVersion: "content-1",
      clock: new ClientClock(0, () => LOCAL_TIME),
    });

    const result = await engine.run();

    expect(MAX_SYNC_BATCH).toBe(500);
    expect(api.requests.map((request) => request.events.length)).toEqual([
      500, 500, 200,
    ]);
    const sent = api.requests.flatMap((request) => request.events);
    expect(sent.map(eventIdOf)).toEqual(eventIds);
    expect(result.accepted).toHaveLength(1200);
    expect(await storage.listPendingOutbox()).toHaveLength(0);
  });

  it("follows cursor and more pages and stores incoming events", async () => {
    const learnerId = "page-learner";
    const storage = await openStorage(learnerId);
    const [incomingId] = ids(1);
    if (!incomingId) {
      throw new Error("missing incoming id");
    }
    const incoming = event(learnerId, incomingId);
    const api = new ScriptedApi((request, index) => {
      if (index === 0) {
        return response(request, {
          events: [incoming.event],
          cursor: 5,
          more: true,
        });
      }
      return response(request, { cursor: 6 });
    });
    const engine = new SyncEngine({
      storage,
      api,
      core: new FakeCore(),
      coreVersion: "0.0.0",
      contentVersion: "content-1",
      clock: new ClientClock(0, () => LOCAL_TIME),
    });

    const result = await engine.run();

    expect(api.requests.map((request) => request.cursor)).toEqual([0, 5]);
    expect(api.requests.map((request) => request.events.length)).toEqual([
      0, 0,
    ]);
    expect(result.received).toBe(1);
    expect(await storage.getCursor()).toBe(6);
    const events = await storage.listEvents();
    expect(events.map((item) => item.event_id)).toEqual([incomingId]);
  });

  it("keeps rejected events in the outbox for diagnostics", async () => {
    const learnerId = "rejected-learner";
    const storage = await openStorage(learnerId);
    const [acceptedId, rejectedId] = ids(2);
    if (!acceptedId || !rejectedId) {
      throw new Error("missing test ids");
    }
    await storage.queueOutbox(event(learnerId, acceptedId));
    await storage.queueOutbox(event(learnerId, rejectedId));
    const api = new ScriptedApi((request) =>
      response(request, {
        accepted: [acceptedId],
        rejected: [{ event_id: rejectedId, reason: "future_event" }],
      }),
    );
    const engine = new SyncEngine({
      storage,
      api,
      core: new FakeCore(),
      coreVersion: "0.0.0",
      contentVersion: "content-1",
      clock: new ClientClock(0, () => LOCAL_TIME),
    });

    await engine.run();

    const outbox = await storage.listOutbox();
    expect(outbox).toHaveLength(1);
    expect(outbox[0]?.event_id).toBe(rejectedId);
    expect(outbox[0]?.status).toBe("rejected");
    expect(outbox[0]?.rejected_reason).toBe("future_event");
    expect(await storage.listPendingOutbox()).toHaveLength(0);
  });

  it("wipes local learner storage immediately on erased", async () => {
    const learnerId = "server-erased-learner";
    const storage = await openStorage(learnerId);
    const [eventId] = ids(1);
    if (!eventId) {
      throw new Error("missing event id");
    }
    await storage.queueOutbox(event(learnerId, eventId));
    const api = new ScriptedApi(() => ({
      erased: true,
      status: "erased",
    }));
    const engine = new SyncEngine({
      storage,
      api,
      core: new FakeCore(),
      coreVersion: "0.0.0",
      contentVersion: "content-1",
      clock: new ClientClock(0, () => LOCAL_TIME),
    });

    const result = await engine.run();
    expect(result.erased).toBe(true);

    const reopened = await openStorage(learnerId);
    expect(await reopened.getSnapshot()).toBeNull();
    expect(await reopened.listOutbox()).toHaveLength(0);
    expect(await reopened.listEvents()).toHaveLength(0);
  });

  it("refolds from scratch when an incoming event precedes the snapshot", async () => {
    const learnerId = "refold-learner";
    const storage = await openStorage(learnerId);
    const [olderId, newerId] = ids(2);
    if (!olderId || !newerId) {
      throw new Error("missing test ids");
    }
    const dictionaryText = "article\n";
    const dictionaryVersion = await sha256Hex(dictionaryText);
    const bundleJson = JSON.stringify({
      format_version: 10,
      content_version: "content-1",
      content: {
        lexicon: {
          version: dictionaryVersion,
          bytes: new TextEncoder().encode(dictionaryText).length,
        },
      },
    });
    const bundleDigest = await sha256Hex(bundleJson);
    const candidate: ContentCandidate = {
      content_version: "content-1",
      core_version: "0.0.0",
      bundle_json: bundleJson,
      bundle_digest: bundleDigest,
      dictionary: { version: dictionaryVersion, text: dictionaryText },
    };
    await storage.stageContent(candidate, async () => true);
    await storage.activateStagedContent();
    await storage.putSnapshot({
      learner_id: learnerId,
      core_version: "0.0.0",
      fold_version: "fold-inputs-1",
      content_version: "content-1",
      last_event_id: newerId,
      incomplete: false,
      state_hash: "old",
      state_json: "{}",
    });
    const incoming = event(learnerId, olderId);
    const api = new ScriptedApi((request) =>
      response(request, { events: [incoming.event], cursor: 1 }),
    );
    const core = new FakeCore();
    const engine = new SyncEngine({
      storage,
      api,
      core,
      coreVersion: "0.0.0",
      contentVersion: "content-1",
      clock: new ClientClock(0, () => LOCAL_TIME),
    });

    const result = await engine.run();

    expect(result.folded).toBe(true);
    expect(core.foldCalls).toHaveLength(1);
    expect(core.foldCalls[0]?.snapshotJson).toBeNull();
    expect(core.foldCalls[0]?.eventCount).toBe(1);
    expect((await storage.getSnapshot())?.last_event_id).toBe(olderId);
  });

  it("persists the server clock offset and flags a large drift", async () => {
    const learnerId = "clock-learner";
    const storage = await openStorage(learnerId);
    const api = new ScriptedApi((request) =>
      response(request, {
        server_time: LOCAL_TIME + 25 * 60 * 60 * 1000,
      }),
    );
    const engine = new SyncEngine({
      storage,
      api,
      core: new FakeCore(),
      coreVersion: "0.0.0",
      contentVersion: "content-1",
      clock: new ClientClock(0, () => LOCAL_TIME),
    });

    const result = await engine.run();
    expect(result.clockSuspect).toBe(true);
    expect(await storage.getClockOffset()).toBe(25 * 60 * 60 * 1000);
  });

  it("does not apply a staged update during a session", async () => {
    const gate = new SessionUpdateGate();
    gate.stageUpdate({ coreVersion: "1.0.0", contentVersion: "content-2" });
    gate.beginSession();
    let applied = false;
    expect(
      await gate.applyStagedUpdate(async () => {
        applied = true;
      }),
    ).toBe(false);
    expect(applied).toBe(false);
    expect(gate.stagedUpdate?.contentVersion).toBe("content-2");

    gate.endSession();
    expect(
      await gate.applyStagedUpdate(async () => {
        applied = true;
      }),
    ).toBe(true);
    expect(applied).toBe(true);
    expect(gate.stagedUpdate).toBeNull();
  });
});
