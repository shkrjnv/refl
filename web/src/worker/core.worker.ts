import { isJsonObject, isJsonValue, type JsonValue } from "../types/json";
import { loadWasmCore } from "../wasm/core";
import type { WasmContext, WasmCore } from "../wasm/types";
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

/**
 * The parsed bundle and lexicon live here between calls. The client sends
 * their text only when it changes; every other request reuses the context.
 */
class ContentCache {
  #bundle: string | null = null;
  #lexicon: string | null = null;
  #context: WasmContext | null = null;
  #error: CoreCallError | null = null;

  context(core: WasmCore, bundleJson?: string, lexiconJson?: string): WasmContext {
    let changed = false;
    if (bundleJson !== undefined && bundleJson !== this.#bundle) {
      this.#bundle = bundleJson;
      changed = true;
    }
    if (lexiconJson !== undefined && lexiconJson !== this.#lexicon) {
      this.#lexicon = lexiconJson;
      changed = true;
    }
    if (this.#bundle === null) {
      throw new CoreCallError("no_content", "the worker has no content bundle yet");
    }
    if (changed || (this.#context === null && this.#error === null)) {
      this.#context?.free();
      this.#context = null;
      this.#error = null;
      try {
        this.#context = core.createContext(this.#bundle, this.#lexicon);
      } catch (error) {
        const text = typeof error === "string" ? error : String(error);
        const split = text.indexOf(": ");
        this.#error =
          split > 0
            ? new CoreCallError(text.slice(0, split), text.slice(split + 2))
            : new CoreCallError("invalid_bundle", text);
      }
    }
    if (this.#error !== null) {
      throw this.#error;
    }
    if (this.#context === null) {
      throw new CoreCallError("no_content", "the worker has no content bundle yet");
    }
    return this.#context;
  }
}

const content = new ContentCache();

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
        content
          .context(core, request.bundleJson, request.lexiconJson)
          .foldState(request.snapshotJson, JSON.stringify(request.events)),
      );
    case "planSession":
      return parseWasm(
        content
          .context(core, request.bundleJson)
          .planSession(request.snapshotJson, request.nowMs, request.budgetMs),
      );
    case "nextItem":
      return parseWasm(
        content
          .context(core, request.bundleJson)
          .nextItem(request.snapshotJson, request.planJson, request.nowMs),
      );
    case "replanSession":
      return parseWasm(
        content
          .context(core, request.bundleJson)
          .replanSession(
            request.snapshotJson,
            request.planJson,
            JSON.stringify(request.outcome),
            request.nowMs,
          ),
      );
    case "checkAnswer":
      return parseWasm(
        content
          .context(core, request.bundleJson, request.lexiconJson)
          .checkAnswer(
            JSON.stringify(request.itemRef),
            JSON.stringify(request.response),
            request.elapsedMs,
          ),
      );
    case "placementStart":
      return parseWasm(
        content
          .context(core, request.bundleJson)
          .placementStart(request.snapshotJson, request.subject, request.nowMs),
      );
    case "placementNext":
      return parseWasm(
        content
          .context(core, request.bundleJson)
          .placementNext(request.snapshotJson, request.placementJson, request.nowMs),
      );
    case "placementAnswer":
      return parseWasm(
        content
          .context(core, request.bundleJson)
          .placementAnswer(
            request.snapshotJson,
            request.placementJson,
            request.itemRefJson,
            request.slot,
            request.correct,
            request.nowMs,
          ),
      );
    case "bookCoverage":
      return parseWasm(
        core.bookCoverage(request.bookJson, request.snapshotJson, request.nowMs),
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
