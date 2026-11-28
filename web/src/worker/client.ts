import { parseJsonValue, type JsonValue } from "../types/json";
import type {
  AnswerCheckResult,
  BundleCompatibility,
  CoreInfo,
  DecodedEvent,
  FoldResult,
  NextItemResult,
  PlacementStep,
  PlanSessionResult,
  ReplanSessionResult,
} from "../wasm/types";
import {
  isCoreResponse,
  type CoreRequest,
  type CoreResponse,
} from "./protocol";

export class CoreWorkerError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "CoreWorkerError";
    this.code = code;
  }
}

export interface StagedCoreUpdate {
  coreVersion: string;
  contentVersion: string;
  bundleJson?: string;
  bundleDigest?: string;
}

export class SessionUpdateGate {
  #sessionActive = false;
  #staged: StagedCoreUpdate | null = null;

  get isSessionActive(): boolean {
    return this.#sessionActive;
  }

  get stagedUpdate(): StagedCoreUpdate | null {
    return this.#staged;
  }

  beginSession(): void {
    this.#sessionActive = true;
  }

  endSession(): void {
    this.#sessionActive = false;
  }

  stageUpdate(update: StagedCoreUpdate): void {
    this.#staged = update;
  }

  async applyStagedUpdate(
    apply: (update: StagedCoreUpdate) => Promise<void>,
  ): Promise<boolean> {
    if (this.#sessionActive || this.#staged === null) {
      return false;
    }
    const update = this.#staged;
    await apply(update);
    this.#staged = null;
    return true;
  }
}

interface PendingCall {
  resolve: (value: JsonValue) => void;
  reject: (reason: unknown) => void;
}

export interface CoreWorkerClientOptions {
  worker?: Worker;
  workerUrl?: URL;
}

export class CoreWorkerClient {
  readonly #worker: Worker;
  readonly #pending = new Map<number, PendingCall>();
  #nextId = 1;
  readonly updates = new SessionUpdateGate();

  constructor(options: CoreWorkerClientOptions = {}) {
    if (options.worker) {
      this.#worker = options.worker;
    } else if (options.workerUrl) {
      this.#worker = new Worker(options.workerUrl, { type: "module" });
    } else {
      this.#worker = new Worker(
        new URL("./core.worker.ts", import.meta.url),
        { type: "module" },
      );
    }
    this.#worker.onmessage = (event: MessageEvent<unknown>) => {
      this.#handleMessage(event.data);
    };
    this.#worker.onerror = (event) => {
      this.#rejectAll(
        new CoreWorkerError(
          "worker_error",
          event.message.length > 0 ? event.message : "core worker failed",
        ),
      );
    };
  }

  terminate(): void {
    this.#worker.terminate();
    this.#rejectAll(new CoreWorkerError("terminated", "core worker terminated"));
  }

  coreInfo(): Promise<CoreInfo> {
    return this.#call({ id: this.#id(), method: "coreInfo" });
  }

  checkBundleCompatibility(
    bundleJson: string,
  ): Promise<BundleCompatibility> {
    return this.#call({
      id: this.#id(),
      method: "checkBundleCompatibility",
      bundleJson,
    });
  }

  decodeEvent(event: JsonValue | string): Promise<DecodedEvent> {
    const value =
      typeof event === "string" ? parseJsonValue(event) : event;
    return this.#call({ id: this.#id(), method: "decodeEvent", event: value });
  }

  foldState(
    bundleJson: string,
    lexiconJson: string,
    snapshotJson: string | null,
    events: JsonValue[],
  ): Promise<FoldResult> {
    return this.#call({
      id: this.#id(),
      method: "foldState",
      bundleJson,
      lexiconJson,
      snapshotJson,
      events,
    });
  }

  planSession(
    bundleJson: string,
    snapshotJson: string,
    nowMs: number,
    budgetMs: number,
  ): Promise<PlanSessionResult> {
    return this.#call({
      id: this.#id(),
      method: "planSession",
      bundleJson,
      snapshotJson,
      nowMs,
      budgetMs,
    });
  }

  nextItem(
    bundleJson: string,
    snapshotJson: string,
    planJson: string,
    nowMs: number,
  ): Promise<NextItemResult> {
    return this.#call({
      id: this.#id(),
      method: "nextItem",
      bundleJson,
      snapshotJson,
      planJson,
      nowMs,
    });
  }

  replanSession(
    bundleJson: string,
    snapshotJson: string,
    planJson: string,
    outcome: JsonValue,
    nowMs: number,
  ): Promise<ReplanSessionResult> {
    return this.#call({
      id: this.#id(),
      method: "replanSession",
      bundleJson,
      snapshotJson,
      planJson,
      outcome,
      nowMs,
    });
  }

  checkAnswer(
    bundleJson: string,
    lexiconJson: string,
    itemRef: JsonValue,
    response: JsonValue,
    elapsedMs: number,
  ): Promise<AnswerCheckResult> {
    return this.#call({
      id: this.#id(),
      method: "checkAnswer",
      bundleJson,
      lexiconJson,
      itemRef,
      response,
      elapsedMs,
    });
  }

  placementStart(
    bundleJson: string,
    snapshotJson: string,
    subject: string,
    nowMs: number,
  ): Promise<PlacementStep> {
    return this.#call({
      id: this.#id(),
      method: "placementStart",
      bundleJson,
      snapshotJson,
      subject,
      nowMs,
    });
  }

  placementNext(
    bundleJson: string,
    snapshotJson: string,
    placementJson: string,
    nowMs: number,
  ): Promise<PlacementStep> {
    return this.#call({
      id: this.#id(),
      method: "placementNext",
      bundleJson,
      snapshotJson,
      placementJson,
      nowMs,
    });
  }

  placementAnswer(
    bundleJson: string,
    snapshotJson: string,
    placementJson: string,
    itemRefJson: string,
    slot: number,
    correct: boolean,
    nowMs: number,
  ): Promise<PlacementStep> {
    return this.#call({
      id: this.#id(),
      method: "placementAnswer",
      bundleJson,
      snapshotJson,
      placementJson,
      itemRefJson,
      slot,
      correct,
      nowMs,
    });
  }

  #id(): number {
    const id = this.#nextId;
    this.#nextId += 1;
    return id;
  }

  #call<T>(request: CoreRequest): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      this.#pending.set(request.id, {
        resolve: (value) => resolve(value as T),
        reject,
      });
      try {
        this.#worker.postMessage(request);
      } catch (error) {
        this.#pending.delete(request.id);
        reject(error);
      }
    });
  }

  #handleMessage(data: unknown): void {
    if (!isCoreResponse(data)) {
      return;
    }
    const pending = this.#pending.get(data.id);
    if (!pending) {
      return;
    }
    this.#pending.delete(data.id);
    if (data.ok) {
      pending.resolve(data.result);
    } else {
      pending.reject(new CoreWorkerError(data.error.code, data.error.message));
    }
  }

  #rejectAll(error: Error): void {
    for (const pending of this.#pending.values()) {
      pending.reject(error);
    }
    this.#pending.clear();
  }
}

export type { CoreResponse };
