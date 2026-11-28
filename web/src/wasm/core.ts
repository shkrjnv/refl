import init, * as wasm from "../generated/wasm/refl_core_wasm.js";

import type { WasmContext, WasmCore } from "./types";

interface CoreContextHandle {
  fold_state(snapshot_json: string | null | undefined, events_json: string): string;
  plan_session(snapshot_json: string, now_ms: number, budget_ms: number): string;
  next_item(snapshot_json: string, plan_json: string, now_ms: number): string;
  replan_session(
    snapshot_json: string,
    plan_json: string,
    outcome_json: string,
    now_ms: number,
  ): string;
  check_answer(item_ref_json: string, response_json: string, elapsed_ms: number): string;
  placement_start(snapshot_json: string, subject: string, now_ms: number): string;
  placement_next(snapshot_json: string, placement_json: string, now_ms: number): string;
  placement_answer(
    snapshot_json: string,
    placement_json: string,
    item_ref_json: string,
    slot: number,
    correct: boolean,
    now_ms: number,
  ): string;
  free(): void;
}

type CoreContextClass = new (
  bundle_json: string,
  lexicon_json?: string | null,
) => CoreContextHandle;

function wrapContext(handle: CoreContextHandle): WasmContext {
  return {
    foldState: (snapshot, events) => handle.fold_state(snapshot, events),
    planSession: (snapshot, now, budget) => handle.plan_session(snapshot, now, budget),
    nextItem: (snapshot, plan, now) => handle.next_item(snapshot, plan, now),
    replanSession: (snapshot, plan, outcome, now) =>
      handle.replan_session(snapshot, plan, outcome, now),
    checkAnswer: (itemRef, response, elapsed) =>
      handle.check_answer(itemRef, response, elapsed),
    placementStart: (snapshot, subject, now) =>
      handle.placement_start(snapshot, subject, now),
    placementNext: (snapshot, placement, now) =>
      handle.placement_next(snapshot, placement, now),
    placementAnswer: (snapshot, placement, itemRef, slot, correct, now) =>
      handle.placement_answer(snapshot, placement, itemRef, slot, correct, now),
    free: () => handle.free(),
  };
}

/**
 * Stateless fallback for a core built before `CoreContext` existed: the
 * same answers, with the bundle parsed on every call.
 */
function statelessContext(bundleJson: string, lexiconJson: string | null): WasmContext {
  const lexicon = lexiconJson ?? '""';
  return {
    foldState: (snapshot, events) => wasm.fold_state(bundleJson, lexicon, snapshot, events),
    planSession: (snapshot, now, budget) =>
      wasm.plan_session(bundleJson, snapshot, now, budget),
    nextItem: (snapshot, plan, now) => wasm.next_item(bundleJson, snapshot, plan, now),
    replanSession: (snapshot, plan, outcome, now) =>
      wasm.replan_session(bundleJson, snapshot, plan, outcome, now),
    checkAnswer: (itemRef, response, elapsed) =>
      wasm.check_answer(bundleJson, lexicon, itemRef, response, elapsed),
    placementStart: (snapshot, subject, now) =>
      wasm.placement_start(bundleJson, snapshot, subject, now),
    placementNext: (snapshot, placement, now) =>
      wasm.placement_next(bundleJson, snapshot, placement, now),
    placementAnswer: (snapshot, placement, itemRef, slot, correct, now) =>
      wasm.placement_answer(bundleJson, snapshot, placement, itemRef, slot, correct, now),
    free: () => undefined,
  };
}

export async function loadWasmCore(): Promise<WasmCore> {
  await init();
  const contextClass = (wasm as unknown as { CoreContext?: CoreContextClass }).CoreContext;
  return {
    createContext: (bundleJson, lexiconJson) =>
      contextClass
        ? wrapContext(new contextClass(bundleJson, lexiconJson))
        : statelessContext(bundleJson, lexiconJson),
    coreInfo: wasm.core_info,
    checkBundleCompatibility: wasm.check_bundle_compatibility,
    decodeEvent: wasm.decode_event_json,
    foldState: wasm.fold_state,
    planSession: wasm.plan_session,
    nextItem: wasm.next_item,
    replanSession: wasm.replan_session,
    checkAnswer: wasm.check_answer,
    placementStart: wasm.placement_start,
    placementNext: wasm.placement_next,
    placementAnswer: wasm.placement_answer,
    bookCoverage: wasm.book_coverage,
  };
}
