import { isJsonObject, isJsonValue, type JsonValue } from "../types/json";
import { loadWasmCore } from "../wasm/core";
import type { WasmCore } from "../wasm/types";
import {
  isCoreRequest,
  type CoreRequest,
  type CoreResponse,
} from "./protocol";

class CoreCallError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "CoreCallError";
    this.code = code;
  }
}

function parseWasm(text: string): JsonValue {
  let value: unknown;
  try {
    value = JSON.parse(text) as unknown;
  } catch (error) {
    throw new CoreCallError(
      "invalid_json",
      error instanceof Error ? error.message : "core returned invalid JSON",
    );
  }
  if (!isJsonValue(value)) {
    throw new CoreCallError("invalid_json", "core returned non-JSON data");
  }
  if (isJsonObject(value)) {
    if (value["kind"] === "error" && isJsonObject(value["error"])) {
      const code = value["error"]["code"];
      const message = value["error"]["message"];
      throw new CoreCallError(
        typeof code === "string" ? code : "core_error",
        typeof message === "string" ? message : "core call failed",
      );
    }
    if (isJsonObject(value["error"])) {
      const code = value["error"]["code"];
      const message = value["error"]["message"];
      throw new CoreCallError(
        typeof code === "string" ? code : "core_error",
        typeof message === "string" ? message : "core call failed",
      );
    }
  }
  return value;
}

function callCore(core: WasmCore, request: CoreRequest): JsonValue {
  switch (request.method) {
    case "coreInfo":
      return parseWasm(core.coreInfo());
    case "checkBundleCompatibility":
      return parseWasm(core.checkBundleCompatibility(request.bundleJson));
    case "decodeEvent":
      return parseWasm(core.decodeEvent(JSON.stringify(request.event)));
    case "foldState":
      return parseWasm(
        core.foldState(
          request.bundleJson,
          request.lexiconJson,
          request.snapshotJson,
          JSON.stringify(request.events),
        ),
      );
    case "planSession":
      return parseWasm(
        core.planSession(
          request.bundleJson,
          request.snapshotJson,
          request.nowMs,
          request.budgetMs,
        ),
      );
    case "nextItem":
      return parseWasm(
        core.nextItem(
          request.bundleJson,
          request.snapshotJson,
          request.planJson,
          request.nowMs,
        ),
      );
    case "replanSession":
      return parseWasm(
        core.replanSession(
          request.bundleJson,
          request.snapshotJson,
          request.planJson,
          JSON.stringify(request.outcome),
          request.nowMs,
        ),
      );
    case "checkAnswer":
      return parseWasm(
        core.checkAnswer(
          request.bundleJson,
          request.lexiconJson,
          JSON.stringify(request.itemRef),
          JSON.stringify(request.response),
          request.elapsedMs,
        ),
      );
    case "placementStart":
      return parseWasm(
        core.placementStart(
          request.bundleJson,
          request.snapshotJson,
          request.subject,
          request.nowMs,
        ),
      );
    case "placementNext":
      return parseWasm(
        core.placementNext(
          request.bundleJson,
          request.snapshotJson,
          request.placementJson,
          request.nowMs,
        ),
      );
    case "bookCoverage":
      return parseWasm(
        core.bookCoverage(
          request.bookJson,
          request.snapshotJson,
          request.nowMs,
        ),
      );
    case "placementAnswer":
      return parseWasm(
        core.placementAnswer(
          request.bundleJson,
          request.snapshotJson,
          request.placementJson,
          request.itemRefJson,
          request.slot,
          request.correct,
          request.nowMs,
        ),
      );
  }
}

const corePromise = loadWasmCore();

function postToMain(message: CoreResponse): void {
  (self as unknown as { postMessage(message: CoreResponse): void }).postMessage(
    message,
  );
}

async function handleMessage(value: unknown): Promise<void> {
  if (!isCoreRequest(value)) {
    return;
  }
  try {
    const core = await corePromise;
    const result = callCore(core, value);
    postToMain({ id: value.id, ok: true, result });
  } catch (error) {
    const code = error instanceof CoreCallError ? error.code : "worker_error";
    const message =
      error instanceof Error ? error.message : "core worker failed";
    postToMain({ id: value.id, ok: false, error: { code, message } });
  }
}

self.addEventListener("message", (event: MessageEvent<unknown>) => {
  void handleMessage(event.data);
});
