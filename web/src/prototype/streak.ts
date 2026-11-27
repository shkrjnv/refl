const STREAK_KEY = "refl.streak";

interface StreakRecord {
  day: string;
  days: number;
}

function localDay(now: number): string {
  const date = new Date(now);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function previousDay(day: string): string {
  const date = new Date(`${day}T12:00:00`);
  date.setDate(date.getDate() - 1);
  return localDay(date.getTime());
}

function read(): StreakRecord | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(STREAK_KEY);
    if (!raw) {
      return null;
    }
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      "day" in parsed &&
      "days" in parsed &&
      typeof parsed.day === "string" &&
      typeof parsed.days === "number" &&
      Number.isFinite(parsed.days)
    ) {
      return { day: parsed.day, days: Math.max(1, Math.floor(parsed.days)) };
    }
  } catch {
    return null;
  }
  return null;
}

export function noteActiveDay(now = Date.now()): number {
  const today = localDay(now);
  const stored = read();
  const days =
    stored === null
      ? 1
      : stored.day === today
        ? stored.days
        : stored.day === previousDay(today)
          ? stored.days + 1
          : 1;
  try {
    window.localStorage.setItem(STREAK_KEY, JSON.stringify({ day: today, days }));
  } catch {
    // The streak badge is a convenience; the app still works without storage.
  }
  return days;
}

export function readStreak(now = Date.now()): number {
  const stored = read();
  if (stored === null) {
    return 1;
  }
  const today = localDay(now);
  if (stored.day === today) {
    return stored.days;
  }
  if (stored.day === previousDay(today)) {
    return stored.days;
  }
  return 1;
}
