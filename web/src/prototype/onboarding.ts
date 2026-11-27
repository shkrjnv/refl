import { isJsonObject } from "../types/json";

const TOUR_KEY = "refl.tour";

export interface TourEvent {
  step: string;
  at: number;
}

interface TourState {
  completed: boolean;
  events: TourEvent[];
}

function emptyState(): TourState {
  return { completed: false, events: [] };
}

export function readTourState(): TourState {
  try {
    const raw = window.localStorage.getItem(TOUR_KEY);
    if (!raw) {
      return emptyState();
    }
    const parsed: unknown = JSON.parse(raw);
    if (!isJsonObject(parsed)) {
      return emptyState();
    }
    const events = Array.isArray(parsed["events"])
      ? parsed["events"].flatMap((event): TourEvent[] => {
          if (!isJsonObject(event)) {
            return [];
          }
          const step = event["step"];
          const at = event["at"];
          return typeof step === "string" && typeof at === "number"
            ? [{ step, at }]
            : [];
        })
      : [];
    return { completed: parsed["completed"] === true, events };
  } catch {
    return emptyState();
  }
}

function store(state: TourState): void {
  try {
    window.localStorage.setItem(TOUR_KEY, JSON.stringify(state));
  } catch {
    // Tour completion is a convenience; the UI still works without storage.
  }
}

export function isTourComplete(): boolean {
  return readTourState().completed;
}

export function recordTourStep(step: string, at = Date.now()): void {
  const state = readTourState();
  const event: TourEvent = { step, at };
  store({ ...state, events: [...state.events, event] });
}

export function completeTour(): void {
  const state = readTourState();
  store({ ...state, completed: true });
  recordTourStep("complete");
}

export function resetTour(): void {
  const state = readTourState();
  store({ completed: false, events: state.events });
  recordTourStep("replay");
}

export function tourEvents(): TourEvent[] {
  return readTourState().events;
}

export type { TourState };
