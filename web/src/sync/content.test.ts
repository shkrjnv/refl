import "fake-indexeddb/auto";

import { afterEach, describe, expect, it } from "vitest";

import type { ActiveContentResponse } from "../api/client";
import { LearnerStorage, sha256Hex } from "../db/storage";
import { parseJsonValue } from "../types/json";
import { SessionUpdateGate } from "../worker/client";
import { ContentUpdateManager } from "./content";

const opened: LearnerStorage[] = [];

async function openStorage(learnerId: string): Promise<LearnerStorage> {
  const storage = await LearnerStorage.open(learnerId);
  opened.push(storage);
  return storage;
}

afterEach(async () => {
  for (const storage of opened.splice(0)) {
    await storage.wipe().catch(() => undefined);
  }
});

describe("content updates", () => {
  it("stages a verified pair but waits for the session boundary", async () => {
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
    const active: ActiveContentResponse = {
      content_version: "content-1",
      min_core_version: "0.0.0",
      digest: await sha256Hex(bundleJson),
      bundle: parseJsonValue(bundleJson),
    };
    const storage = await openStorage("content-gate-learner");
    const gate = new SessionUpdateGate();
    const manager = new ContentUpdateManager(storage, gate);

    gate.beginSession();
    await manager.stage(
      active,
      { version: dictionaryVersion, text: dictionaryText },
      async () => true,
    );
    expect(await storage.getActiveContent()).toBeNull();
    expect(await storage.getStagedContent()).not.toBeNull();
    expect(await manager.applyBetweenSessions()).toBe(false);
    expect(await storage.getActiveContent()).toBeNull();
    expect(await storage.getStagedContent()).not.toBeNull();

    gate.endSession();
    expect(await manager.applyBetweenSessions()).toBe(true);
    expect((await storage.getActiveContent())?.content_version).toBe(
      "content-1",
    );
    expect(await storage.getStagedContent()).toBeNull();
  });
});
