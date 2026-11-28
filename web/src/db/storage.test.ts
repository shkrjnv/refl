import "fake-indexeddb/auto";

import { afterEach, describe, expect, it } from "vitest";

import type { JsonObject } from "../types/json";
import { createUuidV7Generator } from "../sync/uuidv7";
import { logoutAndWipe } from "../sync/session";
import {
  LearnerStorage,
  sha256Hex,
  type ContentCandidate,
  type EventRecord,
  type SnapshotRecord,
} from "./storage";

const opened: LearnerStorage[] = [];

async function openStorage(learnerId: string): Promise<LearnerStorage> {
  const storage = await LearnerStorage.open(learnerId);
  opened.push(storage);
  return storage;
}

function ids(count: number): string[] {
  const generator = createUuidV7Generator(
    () => 1_700_000_000_000,
    () => new Uint8Array(8),
  );
  return Array.from({ length: count }, () => generator.next());
}

function event(learnerId: string, eventId: string, confirmed = false): EventRecord {
  const value: JsonObject = {
    schema_version: 1,
    event_id: eventId,
    learner_id: learnerId,
    session_id: null,
    client_ts: 1_700_000_000_000,
    server_ts: null,
    payload: { type: "lesson_started", lesson: "lesson" },
  };
  return {
    learner_id: learnerId,
    event_id: eventId,
    event: value,
    confirmed,
  };
}

function snapshot(learnerId: string, lastEventId: string): SnapshotRecord {
  return {
    learner_id: learnerId,
    core_version: "0.0.0",
    fold_version: "fold-inputs-1",
    content_version: "content-1",
    last_event_id: lastEventId,
    incomplete: false,
    state_hash: "hash",
    state_json: "{}",
  };
}

afterEach(async () => {
  for (const storage of opened.splice(0)) {
    await storage.wipe().catch(() => undefined);
  }
});

describe("learner storage", () => {
  it("keeps separate learners in separate databases", async () => {
    const [firstId, secondId, thirdId] = ids(3);
    if (!firstId || !secondId || !thirdId) {
      throw new Error("missing test ids");
    }
    const first = await openStorage("learner-a");
    const second = await openStorage("learner-b");

    await first.putSnapshot(snapshot("learner-a", firstId));
    await second.putSnapshot(snapshot("learner-b", secondId));
    await first.putEvents([event("learner-a", thirdId)]);

    expect((await first.getSnapshot())?.learner_id).toBe("learner-a");
    expect((await second.getSnapshot())?.learner_id).toBe("learner-b");
    expect(await first.listEvents()).toHaveLength(1);
    expect(await second.listEvents()).toHaveLength(0);
  });

  it("moves outbox events through accepted and rejected states", async () => {
    const [acceptedId, rejectedId] = ids(2);
    if (!acceptedId || !rejectedId) {
      throw new Error("missing test ids");
    }
    const storage = await openStorage("outbox-learner");
    await storage.queueOutbox(event("outbox-learner", acceptedId));
    await storage.queueOutbox(event("outbox-learner", rejectedId));
    expect(await storage.listPendingOutbox()).toHaveLength(2);

    await storage.markOutboxAccepted([acceptedId]);
    await storage.markOutboxRejected(rejectedId, "future_event");

    const pending = await storage.listPendingOutbox();
    expect(pending).toHaveLength(0);
    const outbox = await storage.listOutbox();
    expect(outbox).toHaveLength(1);
    expect(outbox[0]?.status).toBe("rejected");
    expect(outbox[0]?.rejected_reason).toBe("future_event");

    const events = await storage.listEvents();
    expect(events.find((item) => item.event_id === acceptedId)?.confirmed).toBe(
      true,
    );
    expect(events.find((item) => item.event_id === rejectedId)?.confirmed).toBe(
      false,
    );
  });

  it("compacts only confirmed events already folded into the snapshot", async () => {
    const [firstId, secondId, thirdId] = ids(3);
    if (!firstId || !secondId || !thirdId) {
      throw new Error("missing test ids");
    }
    const storage = await openStorage("compact-learner");
    await storage.putSnapshot(snapshot("compact-learner", secondId));
    await storage.putEvents([
      event("compact-learner", firstId, true),
      event("compact-learner", secondId, true),
      event("compact-learner", thirdId, false),
    ]);

    expect(await storage.compactLog()).toBe(2);
    const remaining = await storage.listEvents();
    expect(remaining).toHaveLength(1);
    expect(remaining[0]?.event_id).toBe(thirdId);
  });

  it("wipes the learner database on erasure", async () => {
    const [eventId] = ids(1);
    if (!eventId) {
      throw new Error("missing test id");
    }
    const storage = await openStorage("erased-learner");
    await storage.putSnapshot(snapshot("erased-learner", eventId));
    await storage.queueOutbox(event("erased-learner", eventId));
    await storage.wipe();

    const reopened = await openStorage("erased-learner");
    expect(await reopened.getSnapshot()).toBeNull();
    expect(await reopened.listOutbox()).toHaveLength(0);
  });

  it("removes local learner data on logout, even if the request fails", async () => {
    const storage = await openStorage("logout-learner");
    await storage.putSnapshot(snapshot("logout-learner", "event"));
    const api = {
      logout: async (): Promise<{ logged_out: boolean }> => {
        throw new Error("offline");
      },
    };

    await expect(logoutAndWipe(api, storage)).rejects.toThrow("offline");
    const reopened = await openStorage("logout-learner");
    expect(await reopened.getSnapshot()).toBeNull();
  });

  it("stages and atomically activates a verified bundle and dictionary", async () => {
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

    const storage = await openStorage("content-learner");
    await storage.stageContent(candidate, async () => true);
    expect(await storage.getStagedContent()).not.toBeNull();
    expect(await storage.getActiveContent()).toBeNull();

    await storage.activateStagedContent();
    expect(await storage.getStagedContent()).toBeNull();
    expect((await storage.getActiveContent())?.content_version).toBe(
      "content-1",
    );
  });

  it("refuses content when compatibility or the digest check fails", async () => {
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
    const storage = await openStorage("bad-content-learner");

    await expect(
      storage.stageContent(candidate, async () => false),
    ).rejects.toThrow("not compatible");
    expect(await storage.getStagedContent()).toBeNull();

    await expect(
      storage.stageContent(
        { ...candidate, bundle_digest: "0".repeat(64) },
        async () => true,
      ),
    ).rejects.toThrow("SHA-256");
  });
  it("keeps the auth session across a database reopen and clears on logout data", async () => {
    const learnerId = "session-persist-learner";
    const session = {
      learner_id: learnerId,
      access_token: "access",
      refresh_token: "refresh",
      expires_at: 1_700_000_900_000,
    };
    const first = await LearnerStorage.open(learnerId);
    await first.setAuthSession(session);
    first.close();

    const reopened = await openStorage(learnerId);
    expect(await reopened.getAuthSession()).toEqual(session);
    await reopened.setAuthSession(null);
    expect(await reopened.getAuthSession()).toBeNull();
  });

});
