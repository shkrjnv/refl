import type { JsonValue } from "../types/json";
import type { ItemRef } from "../generated/core/ItemRef";

export interface CoreInfo {
  core_version: string;
  schema_version: number;
  fold_version: string;
  max_bundle_format: number;
}

export interface BundleCompatibility {
  compatible: boolean;
  reason: string | null;
  format_version: number | null;
  min_core_version: string | null;
  core_version: string;
  max_bundle_format: number;
}

export type DecodedEvent =
  | { kind: "event"; event: JsonValue }
  | {
      kind: "unknown";
      event: JsonValue;
      raw: string;
      schema_version: number;
      event_id: string;
      learner_id: string;
    };

export interface FoldResult {
  snapshot: JsonValue;
  state_hash: string;
  incomplete: boolean;
  last_event_id: string | null;
}

export interface PlanSessionResult {
  plan: JsonValue;
  issued: JsonValue;
}

export type NextItemResult =
  | { kind: "lesson"; kc: string; explanation: string }
  | { kind: "item"; item: JsonValue; slot: number; issued: JsonValue }
  | { kind: "finished"; end: JsonValue };

export interface ReplanSessionResult {
  plan: JsonValue;
  rule: JsonValue;
  issued: JsonValue;
}

export type AnswerCheckResult =
  | {
      kind: "checked";
      item: JsonValue;
      slot: number;
      verdict: "correct" | "typo" | "wrong" | "unrecognized";
      misconception: string | null;
      elapsed_ms: number;
    }
  | {
      kind: "not_checked";
      item: JsonValue;
      slot: number;
      elapsed_ms: number;
    };

export interface PlacementItem {
  item: ItemRef;
  slot: number;
}

export interface Placement {
  subject: string;
  asked: PlacementItem[];
  correct: number;
}

export interface PlacementLevel {
  point: number;
  low: number;
  high: number;
}

export type PlacementStep =
  | { kind: "item"; item: ItemRef; slot: number; placement: Placement }
  | {
      kind: "done";
      placement: Placement;
      level: PlacementLevel | null;
      start_level: number;
      answered: number;
      source: "start_from_zero" | "diagnostic";
    }
  | { kind: "unavailable"; placement: Placement };

export interface WasmCore {
  coreInfo(): string;
  checkBundleCompatibility(bundleJson: string): string;
  decodeEvent(eventJson: string): string;
  foldState(
    bundleJson: string,
    lexiconJson: string,
    snapshotJson: string | null,
    eventsJson: string,
  ): string;
  planSession(
    bundleJson: string,
    snapshotJson: string,
    nowMs: number,
    budgetMs: number,
  ): string;
  nextItem(
    bundleJson: string,
    snapshotJson: string,
    planJson: string,
    nowMs: number,
  ): string;
  replanSession(
    bundleJson: string,
    snapshotJson: string,
    planJson: string,
    outcomeJson: string,
    nowMs: number,
  ): string;
  checkAnswer(
    bundleJson: string,
    lexiconJson: string,
    itemRefJson: string,
    responseJson: string,
    elapsedMs: number,
  ): string;
  placementStart(
    bundleJson: string,
    snapshotJson: string,
    subject: string,
    nowMs: number,
  ): string;
  placementNext(
    bundleJson: string,
    snapshotJson: string,
    placementJson: string,
    nowMs: number,
  ): string;
  placementAnswer(
    bundleJson: string,
    snapshotJson: string,
    placementJson: string,
    itemRefJson: string,
    slot: number,
    correct: boolean,
    nowMs: number,
  ): string;
}
