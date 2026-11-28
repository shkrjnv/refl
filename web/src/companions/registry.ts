import type { SendoMood } from "../components/SendoSprite";

export type CompanionId = "iella" | "sendo";

export interface Companion {
  id: CompanionId;
  nameKey: string;
  kickerKey: string;
  descriptionKey: string;
  base: boolean;
  /** The library milestone opens Sendo; the starter book reaches it. */
  unlockKey: string;
}

export const COMPANIONS: Companion[] = [
  {
    id: "iella",
    nameKey: "iella-name",
    kickerKey: "iella-companion",
    descriptionKey: "companion-iella-note",
    base: true,
    unlockKey: "companion-iella-unlock",
  },
  {
    id: "sendo",
    nameKey: "sendo-name",
    kickerKey: "sendo-companion",
    descriptionKey: "companion-sendo-note",
    base: false,
    unlockKey: "companion-sendo-unlock",
  },
];

const SELECTED_KEY = "refl.companion";
const PREVIEW_KEY = "refl.sendo-preview";

export function isCompanionId(value: string | null): value is CompanionId {
  return value === "iella" || value === "sendo";
}

export function readCompanion(): CompanionId {
  try {
    const stored = window.localStorage.getItem(SELECTED_KEY);
    return isCompanionId(stored) ? stored : "iella";
  } catch {
    return "iella";
  }
}

export function storeCompanion(id: CompanionId): void {
  try {
    window.localStorage.setItem(SELECTED_KEY, id);
  } catch {
    // The event log is the durable choice; local storage is a fast start.
  }
}

export function companionById(id: CompanionId): Companion {
  return COMPANIONS.find((companion) => companion.id === id) ?? COMPANIONS[0]!;
}

export function readSendoPreview(): boolean {
  try {
    return window.localStorage.getItem(PREVIEW_KEY) === "1";
  } catch {
    return false;
  }
}

export function storeSendoPreview(on: boolean): void {
  try {
    window.localStorage.setItem(PREVIEW_KEY, on ? "1" : "0");
  } catch {
    // Preview is a development convenience.
  }
}

/** Dev builds and VITE_SENDO_UNLOCKED preview Sendo without a milestone. */
export function sendoUnlocked(coveragePercent: number | null): boolean {
  if (import.meta.env.DEV || import.meta.env.VITE_SENDO_UNLOCKED === "1") {
    return true;
  }
  return coveragePercent !== null && coveragePercent >= 80;
}

export const IDLE_MOOD: SendoMood = "idle";
