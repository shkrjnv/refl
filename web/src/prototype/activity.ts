import type { JsonValue } from "../types/json";
import { isJsonObject } from "../types/json";

export interface ActivityEvent {
  event_id: string;
  event: JsonValue;
}

export interface ActivityDay {
  day: string;
  count: number;
  weekday: number;
}

export interface ActivitySummary {
  days: ActivityDay[];
  total: number;
  bestStreak: number;
  todayCount: number;
}

// Sixteen full weeks keep the GitHub-style grid filling the card while each
// cell stays in the readable 14-18px range on desktop.
export const ACTIVITY_DAYS = 112;

export function localDay(now = Date.now()): string {
  const date = new Date(now);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function dayFromDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function countAnswerEvents(
  events: readonly ActivityEvent[],
  counts: Map<string, number>,
  answerDays: Set<string>,
): void {
  const seen = new Set<string>();
  for (const record of events) {
    if (seen.has(record.event_id)) {
      continue;
    }
    seen.add(record.event_id);
    if (!isJsonObject(record.event)) {
      continue;
    }
    const payload = record.event["payload"];
    const stamp = record.event["client_ts"];
    if (
      !isJsonObject(payload) ||
      payload["type"] !== "answer" ||
      typeof stamp !== "number" ||
      !Number.isFinite(stamp)
    ) {
      continue;
    }
    const day = localDay(stamp);
    // A placement answer is evidence for the model, not learning activity.
    answerDays.add(day);
    if (payload["context"] === "placement") {
      continue;
    }
    counts.set(day, (counts.get(day) ?? 0) + 1);
  }
}

function countStateDays(
  stateJson: string | null,
  counts: Map<string, number>,
  answerDays: ReadonlySet<string>,
): void {
  if (!stateJson) {
    return;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return;
  }
  if (!isJsonObject(parsed) || !isJsonObject(parsed["last_answer"])) {
    return;
  }
  const touched = new Set<string>();
  for (const stamp of Object.values(parsed["last_answer"])) {
    if (typeof stamp !== "number" || !Number.isFinite(stamp)) {
      continue;
    }
    const day = localDay(stamp);
    if (!counts.has(day) && !touched.has(day) && !answerDays.has(day)) {
      touched.add(day);
      counts.set(day, 1);
    }
  }
}

function bestStreak(days: readonly ActivityDay[]): number {
  let best = 0;
  let current = 0;
  for (const day of days) {
    if (day.count > 0) {
      current += 1;
      best = Math.max(best, current);
    } else {
      current = 0;
    }
  }
  return best;
}

export function activitySummary(
  events: readonly ActivityEvent[],
  stateJson: string | null,
  now = Date.now(),
): ActivitySummary {
  const counts = new Map<string, number>();
  const answerDays = new Set<string>();
  countAnswerEvents(events, counts, answerDays);
  countStateDays(stateJson, counts, answerDays);
  const days: ActivityDay[] = [];
  const cursor = new Date(now);
  cursor.setHours(12, 0, 0, 0);
  cursor.setDate(cursor.getDate() - cursor.getDay() - (ACTIVITY_DAYS - 7));
  for (let index = 0; index < ACTIVITY_DAYS; index += 1) {
    const day = dayFromDate(cursor);
    days.push({
      day,
      count: counts.get(day) ?? 0,
      weekday: cursor.getDay(),
    });
    cursor.setDate(cursor.getDate() + 1);
  }
  const total = days.reduce((sum, day) => sum + day.count, 0);
  const today = localDay(now);
  return {
    days,
    total,
    bestStreak: bestStreak(days),
    todayCount: counts.get(today) ?? 0,
  };
}
