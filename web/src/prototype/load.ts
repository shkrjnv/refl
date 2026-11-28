import type {
  ActiveContentResponse,
  ApiClient,
  LexiconResponse,
} from "../api/client";
import {
  sha256Hex,
  type ContentRecord,
  type DictionaryFile,
  type LearnerStorage,
} from "../db/storage";
import type { ContentUpdateManager } from "../sync/content";
import { isJsonObject, type JsonValue } from "../types/json";
import { PrototypeContent } from "./content";

export interface LoadedContent {
  content: PrototypeContent;
  updateReady: boolean;
  fromCache: boolean;
}

export interface LexiconRef {
  version: string;
  words: number;
  bytes: number;
}

export interface ActiveContentPair {
  active: ActiveContentResponse;
  dictionary: DictionaryFile;
  bundleJson: string;
  lexicon: LexiconRef;
}

function positiveInteger(value: JsonValue | undefined): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

/** The bundle's reference to the exact dictionary it must be paired with. */
export function bundleLexiconRef(bundle: JsonValue): LexiconRef {
  if (!isJsonObject(bundle) || !isJsonObject(bundle["content"])) {
    throw new Error("content bundle has no content object");
  }
  const ref = bundle["content"]["lexicon"];
  if (
    !isJsonObject(ref) ||
    typeof ref["version"] !== "string" ||
    !/^[0-9a-f]{64}$/.test(ref["version"]) ||
    !positiveInteger(ref["words"]) ||
    !positiveInteger(ref["bytes"])
  ) {
    throw new Error("bundle lexicon metadata is invalid");
  }
  return {
    version: ref["version"],
    words: ref["words"],
    bytes: ref["bytes"],
  };
}

function verifyDictionary(
  lexicon: LexiconRef,
  dictionary: LexiconResponse,
): DictionaryFile {
  if (dictionary.version !== lexicon.version) {
    throw new Error("dictionary version does not match the bundle");
  }
  if (dictionary.sha256 !== lexicon.version) {
    throw new Error("dictionary SHA-256 does not match the bundle");
  }
  if (dictionary.bytes !== lexicon.bytes) {
    throw new Error("dictionary size does not match the bundle");
  }
  if (dictionary.words !== lexicon.words) {
    throw new Error("dictionary word count does not match the bundle");
  }
  const encoded = new TextEncoder().encode(dictionary.text);
  if (
    dictionary.text.length === 0 ||
    !dictionary.text.endsWith("\n") ||
    encoded.length !== dictionary.bytes ||
    dictionary.text.split("\n").filter(Boolean).length !== dictionary.words
  ) {
    throw new Error("dictionary contents do not match their metadata");
  }
  return { version: dictionary.version, text: dictionary.text };
}

/**
 * Downloads the active bundle and its exact SCOWL dictionary. The pair is not
 * accepted until the dictionary's SHA-256 and size match the bundle reference.
 */
export async function fetchActiveContentPair(
  api: Pick<ApiClient, "activeContent" | "lexicon">,
  active: ActiveContentResponse | null = null,
): Promise<ActiveContentPair> {
  const response = active ?? (await api.activeContent());
  const bundleJson = response.bundle_json;
  if ((await sha256Hex(bundleJson)) !== response.digest.toLowerCase()) {
    throw new Error("active bundle SHA-256 does not match the server digest");
  }
  let parsed: JsonValue;
  try {
    parsed = JSON.parse(bundleJson) as JsonValue;
  } catch {
    throw new Error("active bundle is not valid JSON");
  }
  const lexicon = bundleLexiconRef(parsed);
  const dictionary = await api.lexicon(lexicon.version);
  if ((await sha256Hex(dictionary.text)) !== lexicon.version) {
    throw new Error("downloaded dictionary SHA-256 does not match its version");
  }
  return {
    active: response,
    dictionary: verifyDictionary(lexicon, dictionary),
    bundleJson,
    lexicon,
  };
}

export function contentFromRecord(record: ContentRecord): PrototypeContent {
  return new PrototypeContent(
    record.bundle_json,
    record.content_version,
    record.bundle_digest,
    record.dictionary.text,
  );
}

export function contentFromPair(pair: ActiveContentPair): PrototypeContent {
  return new PrototypeContent(
    pair.bundleJson,
    pair.active.content_version,
    pair.active.digest,
    pair.dictionary.text,
  );
}

/**
 * Loads a verified active pair. A cached active pair is used immediately; on a
 * first run the bundle and dictionary are downloaded together, staged and
 * activated, never one without the other.
 */
export async function loadPrototypeContent(
  api: Pick<ApiClient, "activeContent" | "lexicon">,
  storage: LearnerStorage,
  manager: ContentUpdateManager,
  verifyCompatibility: (bundleJson: string) => Promise<boolean>,
): Promise<LoadedContent> {
  const active = await storage.getActiveContent();
  if (active) {
    return {
      content: contentFromRecord(active),
      updateReady: false,
      fromCache: true,
    };
  }

  const pair = await fetchActiveContentPair(api);
  await manager.stage(pair.active, pair.dictionary, verifyCompatibility);
  if (!(await manager.applyBetweenSessions())) {
    throw new Error("the content update cannot activate before a session");
  }
  const installed = await storage.getActiveContent();
  if (!installed) {
    throw new Error("the content pair was not activated");
  }
  return {
    content: contentFromRecord(installed),
    updateReady: false,
    fromCache: false,
  };
}
