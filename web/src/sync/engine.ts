import {
  isErasedResponse,
  type ErasedResponse,
  type RejectedEvent,
  type SyncRequest,
  type SyncResponse,
} from "../api/client";
import type { EventRecord, LearnerStorage, SnapshotRecord } from "../db/storage";
import { isJsonObject, type JsonValue } from "../types/json";
import type {
  CoreInfo,
  DecodedEvent,
  FoldResult,
} from "../wasm/types";
import { ClientClock } from "./uuidv7";

export const MAX_SYNC_BATCH = 500;

export interface SyncCore {
  coreInfo(): Promise<CoreInfo>;
  decodeEvent(event: JsonValue): Promise<DecodedEvent>;
  foldState(
    bundleJson: string,
    lexiconJson: string,
    snapshotJson: string | null,
    events: JsonValue[],
  ): Promise<FoldResult>;
}

export interface SyncApi {
  sync(request: SyncRequest): Promise<SyncResponse | ErasedResponse>;
}

export interface SyncEngineOptions {
  storage: LearnerStorage;
  api: SyncApi;
  core: SyncCore;
  coreVersion: string;
  contentVersion: string;
  clock?: ClientClock;
}

export interface SyncResult {
  accepted: string[];
  rejected: RejectedEvent[];
  received: number;
  erased: boolean;
  folded: boolean;
  compacted: number;
  cursor: number;
  clockSuspect: boolean;
}

interface IncomingOutcome {
  received: number;
  needsFullReplay: boolean;
}

export class SyncEngine {
  readonly #storage: LearnerStorage;
  readonly #api: SyncApi;
  readonly #core: SyncCore;
  readonly #coreVersion: string;
  readonly #contentVersion: string;
  readonly #clock: ClientClock;

  constructor(options: SyncEngineOptions) {
    this.#storage = options.storage;
    this.#api = options.api;
    this.#core = options.core;
    this.#coreVersion = options.coreVersion;
    this.#contentVersion = options.contentVersion;
    this.#clock = options.clock ?? new ClientClock();
  }

  async run(): Promise<SyncResult> {
    this.#clock.setOffset(await this.#storage.getClockOffset());
    const snapshot = await this.#storage.getSnapshot();
    const snapshotPosition = snapshot?.last_event_id ?? null;
    const accepted: string[] = [];
    const rejected: RejectedEvent[] = [];
    let received = 0;
    let needsFullReplay = false;
    let cursor = await this.#storage.getCursor();
    let clockSuspect = false;

    let outgoing = await this.#storage.listPendingOutbox(MAX_SYNC_BATCH);
    let sentEvents = outgoing.map((record) => record.event);

    for (;;) {
      const request: SyncRequest = {
        learner_id: this.#storage.learnerId,
        core_version: this.#coreVersion,
        content_version: this.#contentVersion,
        cursor,
        events: sentEvents,
      };
      const response = await this.#api.sync(request);
      if (isErasedResponse(response)) {
        await this.#storage.wipe();
        return {
          accepted,
          rejected,
          received,
          erased: true,
          folded: false,
          compacted: 0,
          cursor,
          clockSuspect,
        };
      }

      const offset = this.#clock.update(response.server_time);
      clockSuspect = clockSuspect || offset.suspicious;
      await this.#storage.setClockOffset(offset.offsetMs);

      if (response.accepted.length > 0) {
        await this.#storage.markOutboxAccepted(response.accepted);
        accepted.push(...response.accepted);
      }
      for (const rejection of response.rejected) {
        await this.#storage.markOutboxRejected(
          rejection.event_id,
          rejection.reason,
        );
      }
      rejected.push(...response.rejected);

      const incoming = await this.#handleIncoming(
        response.events,
        snapshotPosition,
      );
      received += incoming.received;
      needsFullReplay = needsFullReplay || incoming.needsFullReplay;

      cursor = response.cursor;
      await this.#storage.setCursor(cursor);

      if (response.more) {
        sentEvents = [];
        continue;
      }

      outgoing = await this.#storage.listPendingOutbox(MAX_SYNC_BATCH);
      if (outgoing.length > 0) {
        sentEvents = outgoing.map((record) => record.event);
        continue;
      }
      break;
    }

    const fold = await this.#fold(needsFullReplay || snapshot === null);
    return {
      accepted,
      rejected,
      received,
      erased: false,
      folded: fold.folded,
      compacted: fold.compacted,
      cursor,
      clockSuspect,
    };
  }

  async #handleIncoming(
    events: JsonValue[],
    snapshotPosition: string | null,
  ): Promise<IncomingOutcome> {
    let needsFullReplay = false;
    let received = 0;
    const known: EventRecord[] = [];
    const unknown: Array<{
      learner_id: string;
      event_id: string;
      raw: string;
      saved_at: number;
    }> = [];

    for (const event of events) {
      if (!isJsonObject(event)) {
        continue;
      }
      const eventId = event["event_id"];
      const learnerId = event["learner_id"];
      if (
        typeof eventId !== "string" ||
        typeof learnerId !== "string" ||
        learnerId !== this.#storage.learnerId
      ) {
        continue;
      }
      received += 1;
      const decoded = await this.#core.decodeEvent(event);
      if (decoded.kind === "unknown") {
        unknown.push({
          learner_id: learnerId,
          event_id: eventId,
          raw: JSON.stringify(event),
          saved_at: Date.now(),
        });
      } else {
        known.push({
          learner_id: learnerId,
          event_id: eventId,
          event: decoded.event,
          confirmed: true,
        });
      }
      if (snapshotPosition !== null && eventId <= snapshotPosition) {
        needsFullReplay = true;
      }
    }

    await this.#storage.putEvents(known);
    await this.#storage.putUnknownEvents(unknown);
    return { received, needsFullReplay };
  }

  async #fold(
    fullReplay: boolean,
  ): Promise<{ folded: boolean; compacted: number }> {
    const active = await this.#storage.getActiveContent();
    if (!active) {
      return { folded: false, compacted: 0 };
    }
    const snapshot = await this.#storage.getSnapshot();
    const useFullReplay = fullReplay || snapshot === null;
    const events = useFullReplay
      ? await this.#storage.listEvents()
      : await this.#storage.eventsAfter(snapshot.last_event_id);
    if (!useFullReplay && events.length === 0) {
      return { folded: false, compacted: 0 };
    }

    const info = await this.#core.coreInfo();
    const folded = await this.#core.foldState(
      active.bundle_json,
      active.dictionary.text,
      useFullReplay ? null : snapshot.state_json,
      events.map((event) => event.event),
    );
    const unknown = await this.#storage.listUnknownEvents();
    const record: SnapshotRecord = {
      learner_id: this.#storage.learnerId,
      core_version: info.core_version,
      fold_version: info.fold_version,
      content_version: active.content_version,
      last_event_id:
        typeof folded.last_event_id === "string"
          ? folded.last_event_id
          : (snapshot?.last_event_id ?? ""),
      incomplete: folded.incomplete || unknown.length > 0,
      state_hash: folded.state_hash,
      state_json: JSON.stringify(folded.snapshot),
    };
    await this.#storage.putSnapshot(record);
    const compacted = await this.#storage.compactLog();
    return { folded: true, compacted };
  }
}
