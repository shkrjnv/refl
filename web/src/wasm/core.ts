import init, {
  check_answer,
  check_bundle_compatibility,
  core_info,
  decode_event_json,
  fold_state,
  next_item,
  plan_session,
  replan_session,
} from "../generated/wasm/refl_core_wasm.js";

import type { WasmCore } from "./types";

export async function loadWasmCore(): Promise<WasmCore> {
  await init();
  return {
    coreInfo: core_info,
    checkBundleCompatibility: check_bundle_compatibility,
    decodeEvent: decode_event_json,
    foldState: fold_state,
    planSession: plan_session,
    nextItem: next_item,
    replanSession: replan_session,
    checkAnswer: check_answer,
  };
}
