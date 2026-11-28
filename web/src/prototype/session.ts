import type { LearnerStorage } from "../db/storage";
import type { ClientClock } from "../sync/uuidv7";
import type { JsonObject, JsonValue } from "../types/json";
import type { CoreInfo, FoldResult } from "../wasm/types";
import type { PrototypeContent } from "./content";

/**
 * Marks the interval in which a bundle or model update must not be applied.
 * The browser bridge owns the real `SessionUpdateGate`; this narrow shape
 * keeps the lesson and placement runners testable without a worker.
 */
export interface SessionGate {
  beginSession(): void;
  endSession(): void;
}

/** The narrow core surface shared by the entry step and the placement runner. */
export interface SessionCore {
  coreInfo(): Promise<CoreInfo>;
  foldState(
    bundleJson: string,
    lexiconJson: string,
    snapshotJson: string | null,
    events: JsonValue[],
  ): Promise<FoldResult>;
}

export interface SessionOptions {
  storage: LearnerStorage;
  core: SessionCore;
  content: PrototypeContent;
  learnerId: string;
  clock: ClientClock;
  gate?: SessionGate;
}

export function buildEvent(
  learnerId: string,
  clock: ClientClock,
  payload: JsonObject,
  sessionId: string | null = null,
): JsonObject {
  const stamp = clock.stamp();
  return {
    schema_version: 2,
    event_id: stamp.event_id,
    learner_id: learnerId,
    session_id: sessionId,
    client_ts: stamp.client_ts,
    server_ts: null,
    payload,
  };
}

async function saveSnapshot(
  options: SessionOptions,
  folded: FoldResult,
  eventId: string,
): Promise<void> {
  const info = await options.core.coreInfo();
  await options.storage.putSnapshot({
    learner_id: options.learnerId,
    core_version: info.core_version,
    fold_version: info.fold_version,
    content_version: options.content.contentVersion,
    last_event_id: folded.last_event_id ?? eventId,
    incomplete: folded.incomplete,
    state_hash: folded.state_hash,
    state_json: JSON.stringify(folded.snapshot),
  });
}

/** Queues one event, folds it into the snapshot and persists the result. */
export async function appendEvent(
  options: SessionOptions,
  stateJson: string | null,
  event: JsonObject,
): Promise<string> {
  const eventId =
    typeof event["event_id"] === "string" ? event["event_id"] : "";
  await options.storage.queueOutbox({
    learner_id: options.learnerId,
    event_id: eventId,
    event,
    confirmed: false,
  });
  const folded = await options.core.foldState(
    options.content.bundleJson,
    options.content.lexiconJson,
    stateJson,
    [event],
  );
  await saveSnapshot(options, folded, eventId);
  return JSON.stringify(folded.snapshot);
}

/** Folds an empty log once when the learner has no snapshot yet. */
export async function ensureInitialState(
  options: SessionOptions,
): Promise<string> {
  const folded = await options.core.foldState(
    options.content.bundleJson,
    options.content.lexiconJson,
    null,
    [],
  );
  await saveSnapshot(options, folded, "");
  return JSON.stringify(folded.snapshot);
}
