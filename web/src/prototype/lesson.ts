import type {
  AnswerCheckResult,
  CoreInfo,
  FoldResult,
  NextItemResult,
  PlanSessionResult,
  ReplanSessionResult,
} from "../wasm/types";
import type { Explanation } from "../generated/core/Explanation";
import type { ItemRef } from "../generated/core/ItemRef";
import type { LearnerStorage } from "../db/storage";
import type { Locale } from "../i18n";
import { isJsonObject, type JsonObject, type JsonValue } from "../types/json";
import { ClientClock } from "../sync/uuidv7";
import { buildItemView, type ItemView } from "./item";
import type { PrototypeContent } from "./content";

export interface LessonCore {
  coreInfo(): Promise<CoreInfo>;
  planSession(
    bundleJson: string,
    snapshotJson: string,
    nowMs: number,
    budgetMs: number,
  ): Promise<PlanSessionResult>;
  nextItem(
    bundleJson: string,
    snapshotJson: string,
    planJson: string,
    nowMs: number,
  ): Promise<NextItemResult>;
  replanSession(
    bundleJson: string,
    snapshotJson: string,
    planJson: string,
    outcome: JsonValue,
    nowMs: number,
  ): Promise<ReplanSessionResult>;
  checkAnswer(
    bundleJson: string,
    lexiconJson: string,
    itemRef: JsonValue,
    response: JsonValue,
    elapsedMs: number,
  ): Promise<AnswerCheckResult>;
  foldState(
    bundleJson: string,
    lexiconJson: string,
    snapshotJson: string | null,
    events: JsonValue[],
  ): Promise<FoldResult>;
}

export type LessonPhase =
  | "pretest"
  | "explanation"
  | "practice"
  | "feedback"
  | "summary";

export interface LessonFeedback {
  correct: boolean;
  verdict: string;
  why: string;
  misconception: string | null;
  correctAnswer: string;
}

export interface LessonExplanation {
  id: string;
  kcId: string;
  title: string;
  body: string;
  l1Contrast: string | null;
  examples: string[];
}

export interface LessonSummary {
  answered: number;
  correct: number;
  accuracy: number;
  durationMs: number;
  kcTitles: string[];
  masteredKcs: string[];
  openedKcs: string[];
}

export interface LessonView {
  phase: LessonPhase;
  item: ItemView | null;
  explanation: LessonExplanation | null;
  feedback: LessonFeedback | null;
  summary: LessonSummary | null;
  answered: number;
  planned: number;
}

export interface LessonRunnerOptions {
  storage: LearnerStorage;
  core: LessonCore;
  content: PrototypeContent;
  learnerId: string;
  locale: Locale;
  clock?: ClientClock;
  budgetMs?: number;
  sessionId?: string;
}

interface AnswerLog {
  view: ItemView;
  response: JsonValue;
  result: AnswerCheckResult;
}

const DEFAULT_BUDGET_MS = 25 * 60 * 1000;

export function verdictOf(result: AnswerCheckResult): string {
  return result.kind === "checked" ? result.verdict : "unrecognized";
}

export function isCorrect(result: AnswerCheckResult): boolean {
  return (
    result.kind === "checked" &&
    (result.verdict === "correct" || result.verdict === "typo")
  );
}

function emptyView(phase: LessonPhase): LessonView {
  return {
    phase,
    item: null,
    explanation: null,
    feedback: null,
    summary: null,
    answered: 0,
    planned: 0,
  };
}

export class LessonRunner {
  readonly #storage: LearnerStorage;
  readonly #core: LessonCore;
  readonly #content: PrototypeContent;
  readonly #learnerId: string;
  readonly #locale: Locale;
  readonly #clock: ClientClock;
  readonly #budgetMs: number;
  readonly #sessionId: string;
  #coreInfo: CoreInfo | null = null;
  #plan: JsonValue | null = null;
  #planJson = "";
  #stateJson = "";
  #startedAt = 0;
  #shownAt = 0;
  #phase: LessonPhase = "practice";
  #item: ItemView | null = null;
  #explanation: LessonExplanation | null = null;
  #feedback: LessonFeedback | null = null;
  #summary: LessonSummary | null = null;
  #answers: AnswerLog[] = [];
  #initialKcs = new Set<string>();

  constructor(options: LessonRunnerOptions) {
    this.#storage = options.storage;
    this.#core = options.core;
    this.#content = options.content;
    this.#learnerId = options.learnerId;
    this.#locale = options.locale;
    this.#clock = options.clock ?? new ClientClock();
    this.#budgetMs = options.budgetMs ?? DEFAULT_BUDGET_MS;
    this.#sessionId = options.sessionId ?? this.#clock.nextEventId();
  }

  view(): LessonView {
    return {
      phase: this.#phase,
      item: this.#item,
      explanation: this.#explanation,
      feedback: this.#feedback,
      summary: this.#summary,
      answered: this.#answers.length,
      planned: this.#plannedSteps(),
    };
  }

  async start(): Promise<LessonView> {
    this.#clock.setOffset(await this.#storage.getClockOffset());
    this.#coreInfo = await this.#core.coreInfo();
    let snapshot = await this.#storage.getSnapshot();
    if (!snapshot) {
      const folded = await this.#core.foldState(
        this.#content.bundleJson,
        this.#content.lexiconJson,
        null,
        [],
      );
      this.#stateJson = JSON.stringify(folded.snapshot);
      await this.#saveSnapshot(folded, null);
    } else {
      this.#stateJson = snapshot.state_json;
      this.#initialKcs = new Set(Object.keys(this.#kcsObject(this.#stateJson)));
    }
    this.#startedAt = this.#clock.now();
    const plan = await this.#core.planSession(
      this.#content.bundleJson,
      this.#stateJson,
      this.#startedAt,
      this.#budgetMs,
    );
    this.#plan = plan.plan;
    this.#planJson = JSON.stringify(plan.plan);
    await this.#advance();
    return this.view();
  }

  async submitPretest(response: JsonValue): Promise<LessonView> {
    if (this.#phase !== "pretest" || !this.#item) {
      throw new Error("there is no pretest to answer");
    }
    const view = this.#item;
    const result = await this.#check(view, response);
    await this.#logAnswer(view, response, result, "lesson");
    this.#feedback = this.#feedbackFor(view, result);
    this.#phase = "explanation";
    return this.view();
  }

  async continueExplanation(): Promise<LessonView> {
    if (this.#phase !== "explanation") {
      throw new Error("the explanation is not open");
    }
    const adapted = await this.#core.replanSession(
      this.#content.bundleJson,
      this.#stateJson,
      this.#planJson,
      { kind: "lesson" },
      this.#clock.now(),
    );
    this.#setPlan(adapted.plan);
    this.#feedback = null;
    await this.#advance();
    return this.view();
  }

  async submitAnswer(response: JsonValue): Promise<LessonView> {
    if (this.#phase !== "practice" || !this.#item) {
      throw new Error("there is no item to answer");
    }
    const view = this.#item;
    const result = await this.#check(view, response);
    await this.#logAnswer(view, response, result, "practice");
    this.#feedback = this.#feedbackFor(view, result);
    this.#phase = "feedback";
    return this.view();
  }

  async advanceAfterFeedback(): Promise<LessonView> {
    if (this.#phase !== "feedback" || !this.#item || !this.#feedback) {
      throw new Error("there is no feedback to close");
    }
    const view = this.#item;
    const result = this.#answers.at(-1)?.result;
    if (!result) {
      throw new Error("feedback has no answer");
    }
    const adapted = await this.#core.replanSession(
      this.#content.bundleJson,
      this.#stateJson,
      this.#planJson,
      {
        kind: "answer",
        item: view.ref,
        slot: view.slot,
        correct: isCorrect(result),
        rapid: false,
        sure: false,
      },
      this.#clock.now(),
    );
    this.#setPlan(adapted.plan);
    this.#feedback = null;
    await this.#advance();
    return this.view();
  }

  async #advance(): Promise<void> {
    if (!this.#plan || !this.#planJson) {
      throw new Error("the lesson has no plan");
    }
    const next = await this.#core.nextItem(
      this.#content.bundleJson,
      this.#stateJson,
      this.#planJson,
      this.#clock.now(),
    );
    if (next.kind === "lesson") {
      const kc = typeof next.kc === "string" ? next.kc : "";
      const explanation = this.#content.explanation(
        typeof next.explanation === "string" ? next.explanation : "",
      );
      this.#explanation = this.#explanationFrom(explanation, kc);
      const pretest = this.#content.itemForKc(kc, this.#locale);
      if (pretest) {
        const view = buildItemView(
          this.#content,
          pretest.item,
          0,
          this.#locale,
          `${this.#sessionId}:pretest`,
        );
        this.#item = view;
        this.#phase = "pretest";
      } else {
        this.#item = null;
        this.#phase = "explanation";
      }
      this.#shownAt = this.#clock.now();
      return;
    }
    if (next.kind === "item") {
      const ref = next.item as unknown as ItemRef;
      const entry = this.#content.item(ref);
      if (!entry) {
        throw new Error(`item ${ref.id} is not in the bundle`);
      }
      this.#item = buildItemView(
        this.#content,
        entry.item,
        next.slot,
        this.#locale,
        `${this.#sessionId}:${next.issued ? "item" : "session"}`,
      );
      this.#phase = "practice";
      this.#shownAt = this.#clock.now();
      return;
    }
    this.#item = null;
    this.#phase = "summary";
    this.#summary = this.#buildSummary();
  }

  async #check(view: ItemView, response: JsonValue): Promise<AnswerCheckResult> {
    const elapsed = Math.max(0, this.#clock.now() - this.#shownAt);
    return this.#core.checkAnswer(
      this.#content.bundleJson,
      this.#content.lexiconJson,
      view.ref as unknown as JsonValue,
      response,
      elapsed,
    );
  }

  async #logAnswer(
    view: ItemView,
    response: JsonValue,
    result: AnswerCheckResult,
    context: "lesson" | "practice",
  ): Promise<void> {
    const stamp = this.#clock.stamp();
    const misconception =
      result.kind === "checked" ? result.misconception : null;
    const payload: JsonObject = {
      type: "answer",
      item: view.ref as unknown as JsonValue,
      slot: view.slot,
      scaffold: 0,
      revealed_deps: false,
      context,
      response,
      verdict: verdictOf(result),
      misconception: misconception ?? null,
      latency_ms: Math.max(0, this.#clock.now() - this.#shownAt),
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
    const event: JsonObject = {
      schema_version: 2,
      event_id: stamp.event_id,
      learner_id: this.#learnerId,
      session_id: this.#sessionId,
      client_ts: stamp.client_ts,
      server_ts: null,
      payload,
    };
    await this.#storage.queueOutbox({
      learner_id: this.#learnerId,
      event_id: stamp.event_id,
      event,
      confirmed: false,
    });
    const folded = await this.#core.foldState(
      this.#content.bundleJson,
      this.#content.lexiconJson,
      this.#stateJson,
      [event],
    );
    this.#stateJson = JSON.stringify(folded.snapshot);
    await this.#saveSnapshot(folded, stamp.event_id);
    this.#answers.push({ view, response, result });
  }

  async #saveSnapshot(
    folded: FoldResult,
    fallbackEventId: string | null,
  ): Promise<void> {
    if (!this.#coreInfo) {
      throw new Error("core info is missing");
    }
    await this.#storage.putSnapshot({
      learner_id: this.#learnerId,
      core_version: this.#coreInfo.core_version,
      fold_version: this.#coreInfo.fold_version,
      content_version: this.#content.contentVersion,
      last_event_id: folded.last_event_id ?? fallbackEventId ?? "",
      incomplete: folded.incomplete,
      state_hash: folded.state_hash,
      state_json: JSON.stringify(folded.snapshot),
    });
  }

  #explanationFrom(
    explanation: Explanation | null,
    kcId: string,
  ): LessonExplanation {
    return {
      id: explanation?.id ?? kcId,
      kcId,
      title: this.#content.kcTitle(kcId, this.#locale),
      body: explanation?.body ?? "",
      l1Contrast: explanation?.l1_contrast ?? null,
      examples: explanation?.examples.map((example) => example.text) ?? [],
    };
  }

  #feedbackFor(view: ItemView, result: AnswerCheckResult): LessonFeedback {
    const misconception =
      result.kind === "checked" ? result.misconception : null;
    return {
      correct: isCorrect(result),
      verdict: verdictOf(result),
      why: view.why,
      misconception: misconception
        ? (this.#content.misconceptionExplanation(
            misconception,
            this.#locale,
          )?.body ?? null)
        : null,
      correctAnswer: view.correctAnswer,
    };
  }

  #setPlan(plan: JsonValue): void {
    this.#plan = plan;
    this.#planJson = JSON.stringify(plan);
  }

  #plannedSteps(): number {
    const plan = this.#plan;
    if (!isJsonObject(plan)) {
      return 0;
    }
    const steps = plan["steps"];
    const done = plan["done"];
    return (
      (Array.isArray(steps) ? steps.length : 0) +
      (Array.isArray(done) ? done.length : 0)
    );
  }

  #kcsObject(stateJson: string): JsonObject {
    const parsed: unknown = JSON.parse(stateJson);
    if (!isJsonObject(parsed) || !isJsonObject(parsed["kcs"])) {
      return {};
    }
    return parsed["kcs"];
  }

  #buildSummary(): LessonSummary {
    const correct = this.#answers.filter((answer) =>
      isCorrect(answer.result),
    ).length;
    const kcTitles = new Set<string>();
    for (const answer of this.#answers) {
      for (const title of answer.view.kcTitles) {
        kcTitles.add(title);
      }
    }
    const kcs = this.#kcsObject(this.#stateJson);
    const masteredKcs = Object.entries(kcs)
      .filter(([, value]) => isJsonObject(value) && value["ever_mastered"] === true)
      .map(([id]) => this.#content.kcTitle(id, this.#locale));
    const opened = new Set(
      Object.keys(kcs).filter((id) => !this.#initialKcs.has(id)),
    );
    if (isJsonObject(this.#plan) && Array.isArray(this.#plan["learn"])) {
      for (const entry of this.#plan["learn"]) {
        if (isJsonObject(entry) && typeof entry["kc"] === "string") {
          opened.add(entry["kc"]);
        }
      }
    }
    const openedKcs = [...opened].map((id) =>
      this.#content.kcTitle(id, this.#locale),
    );
    return {
      answered: this.#answers.length,
      correct,
      accuracy:
        this.#answers.length === 0
          ? 0
          : Math.round((correct / this.#answers.length) * 100),
      durationMs: Math.max(0, this.#clock.now() - this.#startedAt),
      kcTitles: [...kcTitles],
      masteredKcs,
      openedKcs,
    };
  }
}

export function initialLessonView(): LessonView {
  return emptyView("practice");
}
