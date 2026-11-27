import { isJsonObject, type JsonValue } from "../types/json";

export const OUTBOX_PENDING = "pending";
export const OUTBOX_REJECTED = "rejected";
export type OutboxStatus = typeof OUTBOX_PENDING | typeof OUTBOX_REJECTED;

export interface SnapshotRecord {
  learner_id: string;
  core_version: string;
  fold_version: string;
  content_version: string;
  last_event_id: string;
  incomplete: boolean;
  state_hash: string;
  state_json: string;
}

export interface EventRecord {
  learner_id: string;
  event_id: string;
  event: JsonValue;
  confirmed: boolean;
}

export interface OutboxRecord {
  learner_id: string;
  event_id: string;
  event: JsonValue;
  status: OutboxStatus;
  rejected_reason?: string;
  queued_at: number;
}

export interface UnknownEventRecord {
  learner_id: string;
  event_id: string;
  raw: string;
  saved_at: number;
}

export interface ThemeSelectionRecord {
  theme: string;
  event_id: string;
}

export interface AuthSessionRecord {
  learner_id: string;
  access_token: string;
  refresh_token: string;
  expires_at: number;
}

export interface DictionaryFile {
  version: string;
  text: string;
}

export interface ContentCandidate {
  content_version: string;
  core_version: string;
  bundle_json: string;
  bundle_digest: string;
  dictionary: DictionaryFile;
}

export type ContentSlot = "active" | "staged";

export interface ContentRecord extends ContentCandidate {
  learner_id: string;
  slot: ContentSlot;
  activated_at: number;
}

/** A server bundle cached for offline lessons before a verified pair exists. */
export interface CachedBundleRecord {
  learner_id: string;
  content_version: string;
  core_version: string;
  bundle_json: string;
  digest: string;
  lexicon_text: string;
  cached_at: number;
}

type MetaKey =
  | "cursor"
  | "clock_offset_ms"
  | "auth_session"
  | "theme_selection";

interface MetaRecord {
  key: MetaKey;
  value: JsonValue;
}

const DATABASE_VERSION = 2;
const STORE_NAMES = [
  "snapshots",
  "events",
  "outbox",
  "unknown_events",
  "content",
  "bundle_cache",
  "meta",
] as const;
type StoreName = (typeof STORE_NAMES)[number];

const MAX_BUNDLE_DIGEST_LENGTH = 64;

function databaseName(learnerId: string): string {
  return `refl-learner-${encodeURIComponent(learnerId)}`;
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () =>
      reject(request.error ?? new Error("IndexedDB request failed"));
  });
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onabort = () =>
      reject(transaction.error ?? new Error("IndexedDB transaction aborted"));
    transaction.onerror = () =>
      reject(transaction.error ?? new Error("IndexedDB transaction failed"));
  });
}

async function runTransaction<T>(
  db: IDBDatabase,
  stores: StoreName | StoreName[],
  mode: IDBTransactionMode,
  action: (transaction: IDBTransaction) => Promise<T>,
): Promise<T> {
  const transaction = db.transaction(stores, mode);
  try {
    const result = await action(transaction);
    await transactionDone(transaction);
    return result;
  } catch (error) {
    try {
      transaction.abort();
    } catch {
      // The transaction may already have aborted.
    }
    throw error;
  }
}

function openDatabase(
  name: string,
  factory: IDBFactory,
): Promise<IDBDatabase> {
  return new Promise<IDBDatabase>((resolve, reject) => {
    const request = factory.open(name, DATABASE_VERSION);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains("snapshots")) {
        db.createObjectStore("snapshots", { keyPath: "learner_id" });
      }
      if (!db.objectStoreNames.contains("events")) {
        const events = db.createObjectStore("events", { keyPath: "event_id" });
        events.createIndex("learner_id", "learner_id", { unique: false });
      }
      if (!db.objectStoreNames.contains("outbox")) {
        const outbox = db.createObjectStore("outbox", { keyPath: "event_id" });
        outbox.createIndex("learner_id", "learner_id", { unique: false });
        outbox.createIndex("status", "status", { unique: false });
      }
      if (!db.objectStoreNames.contains("unknown_events")) {
        db.createObjectStore("unknown_events", { keyPath: "event_id" });
      }
      if (!db.objectStoreNames.contains("content")) {
        db.createObjectStore("content", { keyPath: "slot" });
      }
      if (!db.objectStoreNames.contains("bundle_cache")) {
        db.createObjectStore("bundle_cache", { keyPath: "learner_id" });
      }
      if (!db.objectStoreNames.contains("meta")) {
        db.createObjectStore("meta", { keyPath: "key" });
      }
    };
    request.onsuccess = () => {
      const db = request.result;
      db.onversionchange = () => db.close();
      resolve(db);
    };
    request.onerror = () =>
      reject(request.error ?? new Error("Cannot open IndexedDB"));
    request.onblocked = () =>
      reject(new Error("IndexedDB upgrade is blocked by another tab"));
  });
}

function deleteDatabase(name: string, factory: IDBFactory): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    const request = factory.deleteDatabase(name);
    request.onsuccess = () => resolve();
    request.onerror = () =>
      reject(request.error ?? new Error("Cannot delete IndexedDB"));
    request.onblocked = () =>
      reject(new Error("IndexedDB deletion is blocked by another tab"));
  });
}

function checkLearner(record: { learner_id: string }, learnerId: string): void {
  if (record.learner_id !== learnerId) {
    throw new Error(
      `learner storage isolation violated: ${record.learner_id} != ${learnerId}`,
    );
  }
}

function compareEventIds(left: string, right: string): number {
  if (left < right) {
    return -1;
  }
  if (left > right) {
    return 1;
  }
  return 0;
}

function asHex(bytes: ArrayBuffer): string {
  let output = "";
  for (const byte of new Uint8Array(bytes)) {
    output += byte.toString(16).padStart(2, "0");
  }
  return output;
}

export async function sha256Hex(text: string): Promise<string> {
  const subtle = globalThis.crypto?.subtle;
  if (!subtle) {
    throw new Error("Web Crypto is unavailable");
  }
  const digest = await subtle.digest("SHA-256", new TextEncoder().encode(text));
  return asHex(digest);
}

function validDigest(value: string): boolean {
  return (
    value.length === MAX_BUNDLE_DIGEST_LENGTH && /^[0-9a-f]+$/i.test(value)
  );
}

function bundleLexicon(bundleJson: string): {
  version: string;
  bytes: number;
} {
  const value: unknown = JSON.parse(bundleJson);
  if (!isJsonObject(value)) {
    throw new Error("bundle must be a JSON object");
  }
  const content = value["content"];
  if (!isJsonObject(content)) {
    throw new Error("bundle content is missing");
  }
  const lexicon = content["lexicon"];
  if (!isJsonObject(lexicon)) {
    throw new Error("bundle lexicon is missing");
  }
  const version = lexicon["version"];
  const bytes = lexicon["bytes"];
  if (typeof version !== "string" || typeof bytes !== "number") {
    throw new Error("bundle lexicon metadata is invalid");
  }
  return { version, bytes };
}

export class LearnerStorage {
  readonly learnerId: string;
  readonly #db: IDBDatabase;
  readonly #factory: IDBFactory;
  readonly #name: string;

  private constructor(
    learnerId: string,
    db: IDBDatabase,
    factory: IDBFactory,
    name: string,
  ) {
    this.learnerId = learnerId;
    this.#db = db;
    this.#factory = factory;
    this.#name = name;
  }

  static async open(
    learnerId: string,
    factory: IDBFactory = globalThis.indexedDB,
  ): Promise<LearnerStorage> {
    if (learnerId.trim().length === 0) {
      throw new Error("learner_id must not be empty");
    }
    const name = databaseName(learnerId);
    return new LearnerStorage(
      learnerId,
      await openDatabase(name, factory),
      factory,
      name,
    );
  }

  close(): void {
    this.#db.close();
  }

  async wipe(): Promise<void> {
    this.close();
    await deleteDatabase(this.#name, this.#factory);
  }

  async putSnapshot(snapshot: SnapshotRecord): Promise<void> {
    checkLearner(snapshot, this.learnerId);
    await runTransaction(
      this.#db,
      "snapshots",
      "readwrite",
      async (transaction) => {
        await requestResult(
          transaction.objectStore("snapshots").put(snapshot),
        );
      },
    );
  }

  async getSnapshot(): Promise<SnapshotRecord | null> {
    const record = await requestResult<SnapshotRecord | undefined>(
      this.#db
        .transaction("snapshots", "readonly")
        .objectStore("snapshots")
        .get(this.learnerId),
    );
    return record ?? null;
  }

  async putEvent(event: EventRecord): Promise<void> {
    checkLearner(event, this.learnerId);
    await runTransaction(this.#db, ["events"], "readwrite", async (tx) => {
      await requestResult(tx.objectStore("events").put(event));
    });
  }

  async putEvents(events: EventRecord[]): Promise<void> {
    for (const event of events) {
      checkLearner(event, this.learnerId);
    }
    if (events.length === 0) {
      return;
    }
    await runTransaction(this.#db, ["events"], "readwrite", async (tx) => {
      const store = tx.objectStore("events");
      for (const event of events) {
        await requestResult(store.put(event));
      }
    });
  }

  async listEvents(): Promise<EventRecord[]> {
    const events = await requestResult<EventRecord[]>(
      this.#db
        .transaction("events", "readonly")
        .objectStore("events")
        .getAll(),
    );
    return events.sort((left, right) =>
      compareEventIds(left.event_id, right.event_id),
    );
  }

  async eventsAfter(eventId: string | null): Promise<EventRecord[]> {
    const events = await this.listEvents();
    if (eventId === null) {
      return events;
    }
    return events.filter((event) => event.event_id > eventId);
  }

  async queueOutbox(event: EventRecord): Promise<void> {
    checkLearner(event, this.learnerId);
    if (event.confirmed) {
      throw new Error("a queued event cannot already be confirmed");
    }
    const outbox: OutboxRecord = {
      learner_id: this.learnerId,
      event_id: event.event_id,
      event: event.event,
      status: OUTBOX_PENDING,
      queued_at: Date.now(),
    };
    await runTransaction(
      this.#db,
      ["events", "outbox"],
      "readwrite",
      async (tx) => {
        await requestResult(tx.objectStore("events").put(event));
        await requestResult(tx.objectStore("outbox").put(outbox));
      },
    );
  }

  async listPendingOutbox(limit = 500): Promise<OutboxRecord[]> {
    const records = await this.listOutbox();
    return records
      .filter((record) => record.status === OUTBOX_PENDING)
      .slice(0, limit);
  }

  async listOutbox(): Promise<OutboxRecord[]> {
    const records = await requestResult<OutboxRecord[]>(
      this.#db
        .transaction("outbox", "readonly")
        .objectStore("outbox")
        .getAll(),
    );
    return records.sort((left, right) =>
      compareEventIds(left.event_id, right.event_id),
    );
  }

  async markOutboxAccepted(eventIds: string[]): Promise<void> {
    if (eventIds.length === 0) {
      return;
    }
    const accepted = new Set(eventIds);
    await runTransaction(
      this.#db,
      ["outbox", "events"],
      "readwrite",
      async (tx) => {
        const outbox = tx.objectStore("outbox");
        const events = tx.objectStore("events");
        for (const eventId of accepted) {
          await requestResult(outbox.delete(eventId));
          const event = await requestResult<EventRecord | undefined>(
            events.get(eventId),
          );
          if (event) {
            checkLearner(event, this.learnerId);
            await requestResult(events.put({ ...event, confirmed: true }));
          }
        }
      },
    );
  }

  async markOutboxRejected(
    eventId: string,
    reason: string,
  ): Promise<void> {
    await runTransaction(this.#db, "outbox", "readwrite", async (tx) => {
      const store = tx.objectStore("outbox");
      const record = await requestResult<OutboxRecord | undefined>(
        store.get(eventId),
      );
      if (!record) {
        return;
      }
      checkLearner(record, this.learnerId);
      await requestResult(
        store.put({ ...record, status: OUTBOX_REJECTED, rejected_reason: reason }),
      );
    });
  }

  async putUnknownEvents(records: UnknownEventRecord[]): Promise<void> {
    for (const record of records) {
      checkLearner(record, this.learnerId);
    }
    if (records.length === 0) {
      return;
    }
    await runTransaction(
      this.#db,
      "unknown_events",
      "readwrite",
      async (tx) => {
        const store = tx.objectStore("unknown_events");
        for (const record of records) {
          await requestResult(store.put(record));
        }
      },
    );
  }

  async listUnknownEvents(): Promise<UnknownEventRecord[]> {
    const records = await requestResult<UnknownEventRecord[]>(
      this.#db
        .transaction("unknown_events", "readonly")
        .objectStore("unknown_events")
        .getAll(),
    );
    return records.sort((left, right) =>
      compareEventIds(left.event_id, right.event_id),
    );
  }

  async compactLog(): Promise<number> {
    const snapshot = await this.getSnapshot();
    if (!snapshot) {
      return 0;
    }
    const events = await this.listEvents();
    const prunable = events.filter(
      (event) =>
        event.confirmed && event.event_id <= snapshot.last_event_id,
    );
    if (prunable.length === 0) {
      return 0;
    }
    await runTransaction(this.#db, "events", "readwrite", async (tx) => {
      const store = tx.objectStore("events");
      for (const event of prunable) {
        await requestResult(store.delete(event.event_id));
      }
    });
    return prunable.length;
  }

  async setCursor(cursor: number): Promise<void> {
    if (!Number.isInteger(cursor) || cursor < 0) {
      throw new Error("cursor must be a non-negative integer");
    }
    await this.setMeta("cursor", cursor);
  }

  async getCursor(): Promise<number> {
    const value = await this.getMeta("cursor");
    return typeof value === "number" && Number.isInteger(value) && value >= 0
      ? value
      : 0;
  }

  async setClockOffset(offsetMs: number): Promise<void> {
    if (!Number.isFinite(offsetMs)) {
      throw new Error("clock offset must be finite");
    }
    await this.setMeta("clock_offset_ms", Math.round(offsetMs));
  }

  async getClockOffset(): Promise<number> {
    const value = await this.getMeta("clock_offset_ms");
    return typeof value === "number" && Number.isFinite(value) ? value : 0;
  }

  async rememberThemeSelection(
    selection: ThemeSelectionRecord,
  ): Promise<void> {
    const current = await this.getThemeSelection();
    if (
      current !== null &&
      compareEventIds(current.event_id, selection.event_id) >= 0
    ) {
      return;
    }
    await this.setMeta("theme_selection", {
      theme: selection.theme,
      event_id: selection.event_id,
    });
  }

  async getThemeSelection(): Promise<ThemeSelectionRecord | null> {
    const value = await this.getMeta("theme_selection");
    if (!isJsonObject(value)) {
      return null;
    }
    const theme = value["theme"];
    const eventId = value["event_id"];
    if (typeof theme !== "string" || typeof eventId !== "string") {
      return null;
    }
    return { theme, event_id: eventId };
  }

  async setAuthSession(session: AuthSessionRecord | null): Promise<void> {
    if (session === null) {
      await this.deleteMeta("auth_session");
      return;
    }
    checkLearner(session, this.learnerId);
    await this.setMeta("auth_session", {
      learner_id: session.learner_id,
      access_token: session.access_token,
      refresh_token: session.refresh_token,
      expires_at: session.expires_at,
    });
  }

  async getAuthSession(): Promise<AuthSessionRecord | null> {
    const value = await this.getMeta("auth_session");
    if (!isJsonObject(value)) {
      return null;
    }
    const learnerId = value["learner_id"];
    const accessToken = value["access_token"];
    const refreshToken = value["refresh_token"];
    const expiresAt = value["expires_at"];
    if (
      typeof learnerId !== "string" ||
      typeof accessToken !== "string" ||
      typeof refreshToken !== "string" ||
      typeof expiresAt !== "number"
    ) {
      return null;
    }
    if (learnerId !== this.learnerId) {
      return null;
    }
    return {
      learner_id: learnerId,
      access_token: accessToken,
      refresh_token: refreshToken,
      expires_at: expiresAt,
    };
  }

  async stageContent(
    candidate: ContentCandidate,
    verifyCompatibility: (bundleJson: string) => Promise<boolean>,
  ): Promise<void> {
    if (!validDigest(candidate.bundle_digest)) {
      throw new Error("bundle digest must be a SHA-256 hex string");
    }
    if (!validDigest(candidate.dictionary.version)) {
      throw new Error("dictionary version must be a SHA-256 hex string");
    }
    const bundleDigest = await sha256Hex(candidate.bundle_json);
    if (bundleDigest !== candidate.bundle_digest.toLowerCase()) {
      throw new Error("bundle SHA-256 does not match the server digest");
    }
    const dictionaryDigest = await sha256Hex(candidate.dictionary.text);
    if (dictionaryDigest !== candidate.dictionary.version.toLowerCase()) {
      throw new Error("dictionary SHA-256 does not match its version");
    }
    const lexicon = bundleLexicon(candidate.bundle_json);
    if (lexicon.version !== candidate.dictionary.version.toLowerCase()) {
      throw new Error("bundle and dictionary versions do not match");
    }
    if (lexicon.bytes !== new TextEncoder().encode(candidate.dictionary.text).length) {
      throw new Error("bundle and dictionary sizes do not match");
    }
    if (!(await verifyCompatibility(candidate.bundle_json))) {
      throw new Error("bundle is not compatible with this core");
    }
    const record: ContentRecord = {
      learner_id: this.learnerId,
      slot: "staged",
      ...candidate,
      activated_at: Date.now(),
    };
    await runTransaction(this.#db, "content", "readwrite", async (tx) => {
      await requestResult(tx.objectStore("content").put(record));
    });
  }

  async getStagedContent(): Promise<ContentRecord | null> {
    return this.getContent("staged");
  }

  async getActiveContent(): Promise<ContentRecord | null> {
    return this.getContent("active");
  }

  async activateStagedContent(): Promise<void> {
    await runTransaction(this.#db, "content", "readwrite", async (tx) => {
      const store = tx.objectStore("content");
      const staged = await requestResult<ContentRecord | undefined>(
        store.get("staged"),
      );
      if (!staged) {
        throw new Error("no staged content to activate");
      }
      checkLearner(staged, this.learnerId);
      const active: ContentRecord = {
        ...staged,
        slot: "active",
        activated_at: Date.now(),
      };
      await requestResult(store.put(active));
      await requestResult(store.delete("staged"));
    });
  }

  private async getContent(
    slot: ContentSlot,
  ): Promise<ContentRecord | null> {
    const record = await requestResult<ContentRecord | undefined>(
      this.#db.transaction("content", "readonly").objectStore("content").get(slot),
    );
    if (!record) {
      return null;
    }
    checkLearner(record, this.learnerId);
    return record;
  }

  async cacheBundle(record: CachedBundleRecord): Promise<void> {
    checkLearner(record, this.learnerId);
    await runTransaction(
      this.#db,
      "bundle_cache",
      "readwrite",
      async (tx) => {
        await requestResult(tx.objectStore("bundle_cache").put(record));
      },
    );
  }

  async getCachedBundle(): Promise<CachedBundleRecord | null> {
    const record = await requestResult<CachedBundleRecord | undefined>(
      this.#db
        .transaction("bundle_cache", "readonly")
        .objectStore("bundle_cache")
        .get(this.learnerId),
    );
    if (!record) {
      return null;
    }
    checkLearner(record, this.learnerId);
    return record;
  }

  private async getMeta(key: MetaKey): Promise<JsonValue | null> {
    const record = await requestResult<MetaRecord | undefined>(
      this.#db.transaction("meta", "readonly").objectStore("meta").get(key),
    );
    return record?.value ?? null;
  }

  private async setMeta(key: MetaKey, value: JsonValue): Promise<void> {
    const record: MetaRecord = { key, value };
    await runTransaction(this.#db, "meta", "readwrite", async (tx) => {
      await requestResult(tx.objectStore("meta").put(record));
    });
  }

  private async deleteMeta(key: MetaKey): Promise<void> {
    await runTransaction(this.#db, "meta", "readwrite", async (tx) => {
      await requestResult(tx.objectStore("meta").delete(key));
    });
  }
}

export async function requestPersistentStorage(): Promise<
  "granted" | "denied" | "unsupported"
> {
  if (
    typeof navigator === "undefined" ||
    typeof navigator.storage?.persist !== "function"
  ) {
    return "unsupported";
  }
  return (await navigator.storage.persist()) ? "granted" : "denied";
}

export async function isStoragePersisted(): Promise<boolean> {
  if (
    typeof navigator === "undefined" ||
    typeof navigator.storage?.persisted !== "function"
  ) {
    return false;
  }
  return navigator.storage.persisted();
}

export async function deleteLearnerStorage(
  learnerId: string,
  factory: IDBFactory = globalThis.indexedDB,
): Promise<void> {
  await deleteDatabase(databaseName(learnerId), factory);
}
