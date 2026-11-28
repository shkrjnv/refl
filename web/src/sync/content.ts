import type { ActiveContentResponse } from "../api/client";
import type { ContentCandidate, DictionaryFile, LearnerStorage } from "../db/storage";
import type { SessionUpdateGate } from "../worker/client";

export class ContentUpdateManager {
  readonly #storage: LearnerStorage;
  readonly #gate: SessionUpdateGate;

  constructor(storage: LearnerStorage, gate: SessionUpdateGate) {
    this.#storage = storage;
    this.#gate = gate;
  }

  /** A verified pair is waiting for the next session boundary. */
  get updateReady(): boolean {
    return this.#gate.stagedUpdate !== null;
  }

  get stagedContentVersion(): string | null {
    return this.#gate.stagedUpdate?.contentVersion ?? null;
  }

  async stage(
    active: ActiveContentResponse,
    dictionary: DictionaryFile,
    verifyCompatibility: (bundleJson: string) => Promise<boolean>,
  ): Promise<void> {
    const candidate: ContentCandidate = {
      content_version: active.content_version,
      core_version: active.min_core_version,
      bundle_json: active.bundle_json,
      bundle_digest: active.digest,
      dictionary,
    };
    await this.#storage.stageContent(candidate, verifyCompatibility);
    this.#gate.stageUpdate({
      coreVersion: active.min_core_version,
      contentVersion: active.content_version,
      bundleJson: candidate.bundle_json,
      bundleDigest: candidate.bundle_digest,
    });
  }

  async applyBetweenSessions(): Promise<boolean> {
    return this.#gate.applyStagedUpdate(async (update) => {
      if (!update.bundleJson || !update.bundleDigest) {
        throw new Error("staged content update is incomplete");
      }
      await this.#storage.activateStagedContent();
    });
  }
}
