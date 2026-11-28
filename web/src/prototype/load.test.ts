import "fake-indexeddb/auto";

import { afterEach, describe, expect, it } from "vitest";

import type {
  ActiveContentResponse,
  LexiconResponse,
} from "../api/client";
import { LearnerStorage, sha256Hex } from "../db/storage";
import { ContentUpdateManager } from "../sync/content";
import { parseJsonValue } from "../types/json";
import { SessionUpdateGate } from "../worker/client";
import {
  fetchActiveContentPair,
  loadPrototypeContent,
} from "./load";

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

async function pair(): Promise<{
  active: ActiveContentResponse;
  lexicon: LexiconResponse;
}> {
  const text = "article\ncat\n";
  const version = await sha256Hex(text);
  const bundleJson = JSON.stringify({
    format_version: 10,
    content_version: "content-1",
    content: {
      lexicon: { version, words: 2, bytes: new TextEncoder().encode(text).length },
    },
  });
  return {
    active: {
      content_version: "content-1",
      min_core_version: "0.0.0",
      digest: await sha256Hex(bundleJson),
      bundle: parseJsonValue(bundleJson),
      bundle_json: bundleJson,
    },
    lexicon: {
      version,
      sha256: version,
      words: 2,
      bytes: new TextEncoder().encode(text).length,
      text,
      license: "SCOWL notice",
    },
  };
}

describe("bundled content loading", () => {
  it("verifies and activates the exact bundle and dictionary pair", async () => {
    const bundle = await pair();
    const storage = await openStorage("pair-learner");
    const gate = new SessionUpdateGate();
    const manager = new ContentUpdateManager(storage, gate);
    const api = {
      activeContent: async () => bundle.active,
      lexicon: async () => bundle.lexicon,
    };

    const loaded = await loadPrototypeContent(
      api,
      storage,
      manager,
      async () => true,
    );

    expect(loaded.fromCache).toBe(false);
    expect(loaded.content.lexiconText).toBe(bundle.lexicon.text);
    expect((await storage.getActiveContent())?.dictionary.text).toBe(
      bundle.lexicon.text,
    );
    expect(await storage.getStagedContent()).toBeNull();
  });

  it("rejects a dictionary whose SHA-256 does not match its version", async () => {
    const bundle = await pair();
    const api = {
      activeContent: async () => bundle.active,
      lexicon: async () => ({ ...bundle.lexicon, text: "article\ncat\ndog\n" }),
    };
    await expect(fetchActiveContentPair(api)).rejects.toThrow("SHA-256");
  });

  it("rejects a dictionary with mismatched size or word count", async () => {
    const bundle = await pair();
    const api = {
      activeContent: async () => bundle.active,
      lexicon: async () => ({ ...bundle.lexicon, bytes: 999 }),
    };
    await expect(fetchActiveContentPair(api)).rejects.toThrow("size");

    const wrongWords = {
      activeContent: async () => bundle.active,
      lexicon: async () => ({ ...bundle.lexicon, words: 3 }),
    };
    await expect(fetchActiveContentPair(wrongWords)).rejects.toThrow(
      "word count",
    );
  });

  it("rejects a bundle digest mismatch", async () => {
    const bundle = await pair();
    const api = {
      activeContent: async () => ({ ...bundle.active, digest: "0".repeat(64) }),
      lexicon: async () => bundle.lexicon,
    };
    await expect(fetchActiveContentPair(api)).rejects.toThrow("bundle SHA-256");
  });
});
