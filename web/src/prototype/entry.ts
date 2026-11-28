import { isJsonObject, type JsonValue } from "../types/json";
import { appendEvent, buildEvent, type SessionOptions } from "./session";

const ENTRY_KEY = "refl.entry-choice";
const PLACEMENT_KEY = "refl.placement-progress";

export type EntryChoice = "start_from_zero" | "diagnostic";
export type EntryDecision = "needed" | "done";

interface EntryRecord {
  learner_id: string;
  choice: EntryChoice;
}

function readRecord(): EntryRecord | null {
  try {
    const raw = window.localStorage.getItem(ENTRY_KEY);
    if (!raw) {
      return null;
    }
    const parsed: unknown = JSON.parse(raw);
    if (!isJsonObject(parsed)) {
      return null;
    }
    const learnerId = parsed["learner_id"];
    const choice = parsed["choice"];
    if (
      typeof learnerId !== "string" ||
      (choice !== "start_from_zero" && choice !== "diagnostic")
    ) {
      return null;
    }
    return { learner_id: learnerId, choice };
  } catch {
    return null;
  }
}

export function readEntryChoice(learnerId: string): EntryChoice | null {
  const record = readRecord();
  return record?.learner_id === learnerId ? record.choice : null;
}

export function completeEntryChoice(
  learnerId: string,
  choice: EntryChoice,
): void {
  try {
    window.localStorage.setItem(
      ENTRY_KEY,
      JSON.stringify({ learner_id: learnerId, choice }),
    );
  } catch {
    // The folded profile is the durable check; the flag is a convenience.
  }
}

export function resetEntryChoice(learnerId?: string): void {
  try {
    const record = readRecord();
    if (!learnerId || record?.learner_id === learnerId) {
      window.localStorage.removeItem(ENTRY_KEY);
    }
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

function hasProfileStartLevel(stateJson: string | null): boolean {
  if (!stateJson) {
    return false;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return false;
  }
  if (!isJsonObject(parsed) || !isJsonObject(parsed["profile"])) {
    return false;
  }
  const startLevel = parsed["profile"]["start_level"];
  return (
    isJsonObject(startLevel) &&
    typeof startLevel["en"] === "number" &&
    Number.isFinite(startLevel["en"])
  );
}

function hasSavedPlacement(learnerId: string): boolean {
  try {
    const raw = window.localStorage.getItem(PLACEMENT_KEY);
    if (!raw) {
      return false;
    }
    const parsed: unknown = JSON.parse(raw);
    return isJsonObject(parsed) && parsed["learner_id"] === learnerId;
  } catch {
    return false;
  }
}

function clearSavedPlacement(): void {
  try {
    window.localStorage.removeItem(PLACEMENT_KEY);
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

function hasLegacyEvidence(stateJson: string | null): boolean {
  if (!stateJson) {
    return false;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return false;
  }
  if (!isJsonObject(parsed)) {
    return false;
  }
  return (
    (isJsonObject(parsed["kcs"]) &&
      Object.keys(parsed["kcs"]).length > 0) ||
    (isJsonObject(parsed["misconceptions"]) &&
      Object.keys(parsed["misconceptions"]).length > 0) ||
    (isJsonObject(parsed["last_answer"]) &&
      Object.keys(parsed["last_answer"]).length > 0)
  );
}

/** Event types that show a learner has already used the course. */
const LEARNING_EVENTS = new Set([
  "answer",
  "placement_completed",
  "lesson_started",
  "lesson_finished",
  "explanation_viewed",
  "reclassified",
  "mentor_override",
]);

export function hasLearningHistory(
  records: readonly { event: JsonValue }[],
): boolean {
  return records.some((record) => {
    if (!isJsonObject(record.event)) {
      return false;
    }
    const payload = record.event["payload"];
    return (
      isJsonObject(payload) &&
      typeof payload["type"] === "string" &&
      LEARNING_EVENTS.has(payload["type"])
    );
  });
}

/**
 * A synced placement/profile, a local choice, or any legacy evidence means
 * the entry gate is done. Only a learner with no state and no flag is asked.
 */
export function entryDecision(
  stateJson: string | null,
  learnerId: string,
  hasHistory = false,
): EntryDecision {
  if (readEntryChoice(learnerId) !== null) {
    return "done";
  }
  if (hasProfileStartLevel(stateJson)) {
    return "done";
  }
  // Placement answers are real evidence, so an interrupted check has state
  // but no profile. Keep the gate and resume from the saved progress.
  if (hasSavedPlacement(learnerId)) {
    return "needed";
  }
  if (hasHistory || hasLegacyEvidence(stateJson)) {
    return "done";
  }
  return "needed";
}

/** Records the explicit “start from the beginning” choice as an event. */
export async function recordZeroEntry(
  options: SessionOptions,
  stateJson: string | null,
): Promise<string> {
  clearSavedPlacement();
  const event = buildEvent(options.learnerId, options.clock, {
    type: "placement_completed",
    start_level: { en: 0 },
    answered: 0,
    source: "start_from_zero",
  });
  const next = await appendEvent(options, stateJson, event);
  completeEntryChoice(options.learnerId, "start_from_zero");
  return next;
}
