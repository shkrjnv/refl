import { describe, expect, it } from "vitest";

import { CoreWorkerClient } from "./client";
import type { CoreRequest } from "./protocol";

class FakeWorker {
  sent: CoreRequest[] = [];
  failNext = false;
  onmessage: ((event: MessageEvent<unknown>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;

  postMessage(request: CoreRequest): void {
    if (this.failNext) {
      this.failNext = false;
      throw new Error("DataCloneError");
    }
    this.sent.push(request);
    queueMicrotask(() => {
      this.onmessage?.({ data: { id: request.id, ok: true, result: {} } } as MessageEvent<unknown>);
    });
  }

  terminate(): void {}
}

function client(): { core: CoreWorkerClient; worker: FakeWorker } {
  const worker = new FakeWorker();
  const core = new CoreWorkerClient({ worker: worker as unknown as Worker });
  return { core, worker };
}

describe("CoreWorkerClient content transfer", () => {
  it("sends the bundle and lexicon once while they stay the same", async () => {
    const { core, worker } = client();
    await core.foldState("BUNDLE", "LEXICON", null, []);
    await core.planSession("BUNDLE", "{}", 1, 600_000);
    await core.checkAnswer("BUNDLE", "LEXICON", {}, {}, 10);

    expect(worker.sent[0]).toMatchObject({ bundleJson: "BUNDLE", lexiconJson: "LEXICON" });
    expect(worker.sent[1]).not.toHaveProperty("bundleJson");
    expect(worker.sent[2]).not.toHaveProperty("bundleJson");
    expect(worker.sent[2]).not.toHaveProperty("lexiconJson");
  });

  it("sends the text again when the content changes", async () => {
    const { core, worker } = client();
    await core.planSession("V1", "{}", 1, 600_000);
    await core.planSession("V2", "{}", 1, 600_000);
    await core.planSession("V2", "{}", 1, 600_000);

    expect(worker.sent[0]).toMatchObject({ bundleJson: "V1" });
    expect(worker.sent[1]).toMatchObject({ bundleJson: "V2" });
    expect(worker.sent[2]).not.toHaveProperty("bundleJson");
  });

  it("never strips the bundle from a compatibility check", async () => {
    const { core, worker } = client();
    await core.planSession("BUNDLE", "{}", 1, 600_000);
    await core.checkBundleCompatibility("BUNDLE");
    expect(worker.sent[1]).toMatchObject({ bundleJson: "BUNDLE" });
  });

  it("resends the content after a failed post", async () => {
    const { core, worker } = client();
    worker.failNext = true;
    await expect(core.planSession("BUNDLE", "{}", 1, 600_000)).rejects.toThrow();
    await core.planSession("BUNDLE", "{}", 1, 600_000);
    expect(worker.sent[0]).toMatchObject({ bundleJson: "BUNDLE" });
  });
});
