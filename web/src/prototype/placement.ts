import { isJsonObject, type JsonObject, type JsonValue } from "../types/json";
import type { Locale } from "../i18n";
import type { ClientClock } from "../sync/uuidv7";
import type {
  AnswerCheckResult,
  CoreInfo,
  FoldResult,
  Placement,
  PlacementLevel,
  PlacementStep,
} from "../wasm/types";
import { buildItemView, type ItemView } from "./item";
import { isCorrect, verdictOf } from "./lesson";
import { completeEntryChoice } from "./entry";
import type { PrototypeContent } from "./content";
import {
  appendEvent,
  buildEvent,
  ensureInitialState,
  type SessionGate,
  type SessionOptions,
} from "./session";

const PROGRESS_KEY = "refl.placement-progress";
const PLANNED_ITEMS = 10;

export interface PlacementResultView {
  level: PlacementLevel | null;
  startLevel: number;
  answered: number;
  source: "start_from_zero" | "diagnostic";
}

export type PlacementView =
  | { phase: "item"; item: ItemView; answered: number; planned: number }
  | { phase: "result"; result: PlacementResultView }
  | { phase: "unavailable" };

export interface PlacementCore {
  coreInfo(): Promise<CoreInfo>;
  foldState(
    bundleJson: string,
    lexiconJson: string,
    snapshotJson: string | null,
    events: JsonValue[],
  ): Promise<FoldResult>;
  checkAnswer(
    bundleJson: string,
    lexiconJson: string,
    itemRef: JsonValue,
    response: JsonValue,
    elapsedMs: number,
  ): Promise<AnswerCheckResult>;
  placementStart(
    bundleJson: string,
    snapshotJson: string,
    subject: string,
    nowMs: number,
  ): Promise<PlacementStep>;
  placementNext(
    bundleJson: string,
    snapshotJson: string,
    placementJson: string,
    nowMs: number,
  ): Promise<PlacementStep>;
  placementAnswer(
    bundleJson: string,
    snapshotJson: string,
    placementJson: string,
    itemRefJson: string,
    slot: number,
    correct: boolean,
    nowMs: number,
  ): Promise<PlacementStep>;
}

export interface PlacementRunnerOptions {
  storage: SessionOptions["storage"];
  core: PlacementCore;
  content: PrototypeContent;
  learnerId: string;
  locale: Locale;
  clock: ClientClock;
  gate?: SessionGate;
}

interface SavedProgress {
  learner_id: string;
  placement: Placement;
}

function readSaved(learnerId: string): Placement | null {
  try {
    const raw = window.localStorage.getItem(PROGRESS_KEY);
    if (!raw) {
      return null;
    }
    const parsed: unknown = JSON.parse(raw);
    if (!isJsonObject(parsed) || parsed["learner_id"] !== learnerId) {
      return null;
    }
    const placement = parsed["placement"];
    if (
      !isJsonObject(placement) ||
      typeof placement["subject"] !== "string" ||
      !Array.isArray(placement["asked"])
    ) {
      return null;
    }
    return placement as unknown as Placement;
  } catch {
    return null;
  }
}

function saveProgress(learnerId: string, placement: Placement): void {
  try {
    const saved: SavedProgress = { learner_id: learnerId, placement };
    window.localStorage.setItem(PROGRESS_KEY, JSON.stringify(saved));
  } catch {
    // Resume is a convenience; the event log remains the source of truth.
  }
}

function clearProgress(): void {
  try {
    window.localStorage.removeItem(PROGRESS_KEY);
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

export class PlacementRunner {
  readonly #options: SessionOptions;
  readonly #core: PlacementCore;
  readonly #content: PrototypeContent;
  readonly #locale: Locale;
  readonly #sessionId: string;
  readonly #gate: SessionGate | undefined;
  #stateJson = "";
  #placement: Placement | null = null;
  #view: PlacementView | null = null;
  #shownAt = 0;

  constructor(options: PlacementRunnerOptions) {
    this.#options = {
      storage: options.storage,
      core: options.core,
      content: options.content,
      learnerId: options.learnerId,
      clock: options.clock,
    };
    this.#core = options.core;
    this.#content = options.content;
    this.#locale = options.locale;
    this.#sessionId = options.clock.nextEventId();
    this.#gate = options.gate;
  }

  view(): PlacementView {
    if (!this.#view) {
      throw new Error("the placement check has not started");
    }
    return this.#view;
  }

  async start(stateJson: string | null): Promise<PlacementView> {
    this.#gate?.beginSession();
    try {
      this.#options.clock.setOffset(
        await this.#options.storage.getClockOffset(),
      );
      this.#stateJson =
        stateJson ?? (await ensureInitialState(this.#options));
      const saved = readSaved(this.#options.learnerId);
      const step = saved
        ? await this.#core.placementNext(
            this.#content.bundleJson,
            this.#stateJson,
            JSON.stringify(saved),
            this.#options.clock.now(),
          )
        : await this.#core.placementStart(
            this.#content.bundleJson,
            this.#stateJson,
            "en",
            this.#options.clock.now(),
          );
      return this.#handle(step);
    } catch (error) {
      this.#gate?.endSession();
      throw error;
    }
  }

  /** Ends the update lock when the check leaves the screen. */
  finish(): void {
    this.#gate?.endSession();
  }

  async submit(response: JsonValue): Promise<PlacementView> {
    if (!this.#view || this.#view.phase !== "item" || !this.#placement) {
      throw new Error("there is no placement item to answer");
    }
    const view = this.#view.item;
    const result = await this.#core.checkAnswer(
      this.#content.bundleJson,
      this.#content.lexiconJson,
      view.ref as unknown as JsonValue,
      response,
      Math.max(0, this.#options.clock.now() - this.#shownAt),
    );
    const event = buildEvent(
      this.#options.learnerId,
      this.#options.clock,
      this.#answerPayload(view, response, result),
      this.#sessionId,
    );
    this.#stateJson = await appendEvent(
      this.#options,
      this.#stateJson,
      event,
    );
    const step = await this.#core.placementAnswer(
      this.#content.bundleJson,
      this.#stateJson,
      JSON.stringify(this.#placement),
      JSON.stringify(view.ref),
      view.slot,
      isCorrect(result),
      this.#options.clock.now(),
    );
    return this.#handle(step);
  }

  #answerPayload(
    view: ItemView,
    response: JsonValue,
    result: AnswerCheckResult,
  ): JsonObject {
    const misconception =
      result.kind === "checked" ? result.misconception : null;
    return {
      type: "answer",
      item: view.ref as unknown as JsonValue,
      slot: view.slot,
      scaffold: 0,
      revealed_deps: false,
      context: "placement",
      response,
      verdict: verdictOf(result),
      misconception: misconception ?? null,
      latency_ms: Math.max(0, this.#options.clock.now() - this.#shownAt),
      first_input_ms: null,
      typing_ms: null,
      chars_typed: null,
      edits: null,
      input_device: view.kind === "gap_fill" ? "keyboard" : "touch",
      hidden_ms: null,
      confidence: null,
      hints: 0,
      attempt: 1,
    };
  }

  async #handle(step: PlacementStep): Promise<PlacementView> {
    this.#placement = step.placement;
    if (step.kind === "item") {
      const entry = this.#content.item(step.item);
      if (!entry) {
        throw new Error(`item ${step.item.id} is not in the bundle`);
      }
      saveProgress(this.#options.learnerId, step.placement);
      const item = buildItemView(
        this.#content,
        entry.item,
        step.slot,
        this.#locale,
        `${this.#sessionId}:${step.placement.asked.length}`,
      );
      this.#shownAt = this.#options.clock.now();
      this.#view = {
        phase: "item",
        item,
        answered: step.placement.asked.length,
        planned: PLANNED_ITEMS,
      };
      return this.#view;
    }
    if (step.kind === "unavailable") {
      clearProgress();
      this.#view = { phase: "unavailable" };
      return this.#view;
    }
    const event = buildEvent(
      this.#options.learnerId,
      this.#options.clock,
      {
        type: "placement_completed",
        start_level: { en: step.start_level },
        answered: step.answered,
        source: step.source,
      },
    );
    this.#stateJson = await appendEvent(
      this.#options,
      this.#stateJson,
      event,
    );
    completeEntryChoice(this.#options.learnerId, "diagnostic");
    clearProgress();
    this.#view = {
      phase: "result",
      result: {
        level: step.level,
        startLevel: step.start_level,
        answered: step.answered,
        source: step.source,
      },
    };
    return this.#view;
  }
}
