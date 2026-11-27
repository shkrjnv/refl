import type { Locale } from "../i18n";
import { isJsonObject } from "../types/json";
import type { PrototypeContent } from "./content";

export interface CardView {
  stability: number;
  difficulty: number;
  last_review: number;
}

export interface KcStateView {
  ever_mastered: boolean;
  observations: number;
  card: CardView | null;
}

export interface LearnerState {
  kcs: Record<string, KcStateView>;
}

export interface KcProgress {
  kcId: string;
  title: string;
  mastered: boolean;
  observations: number;
  stabilityDays: number;
  percent: number;
  lastReview: number | null;
  due: boolean;
}

export interface StatsSummary {
  mastered: number;
  inMemory: number;
  averageStability: number;
  calibration: number | null;
}

export function parseLearnerState(stateJson: string | null): LearnerState {
  if (!stateJson) {
    return { kcs: {} };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return { kcs: {} };
  }
  if (!isJsonObject(parsed) || !isJsonObject(parsed["kcs"])) {
    return { kcs: {} };
  }
  const kcs: Record<string, KcStateView> = {};
  for (const [id, value] of Object.entries(parsed["kcs"])) {
    if (!isJsonObject(value)) {
      continue;
    }
    const observations = value["observations"];
    const card = value["card"];
    const stability = isJsonObject(card) ? card["stability"] : null;
    const difficulty = isJsonObject(card) ? card["difficulty"] : null;
    const lastReview = isJsonObject(card) ? card["last_review"] : null;
    kcs[id] = {
      ever_mastered: value["ever_mastered"] === true,
      observations:
        typeof observations === "number" && Number.isFinite(observations)
          ? observations
          : 0,
      card:
        typeof stability === "number" &&
        typeof difficulty === "number" &&
        typeof lastReview === "number"
          ? {
              stability,
              difficulty,
              last_review: lastReview,
            }
          : null,
    };
  }
  return { kcs };
}

export function kcProgress(
  content: PrototypeContent,
  state: LearnerState,
  locale: Locale,
  now = Date.now(),
): KcProgress[] {
  return content.visibleArticles(locale).map((entry) => {
    const current = state.kcs[entry.kc.id];
    const mastered = current?.ever_mastered === true;
    const observations = current?.observations ?? 0;
    const stabilityDays = current?.card?.stability ?? 0;
    const lastReview = current?.card?.last_review ?? null;
    const due =
      lastReview !== null &&
      stabilityDays > 0 &&
      now - lastReview >= stabilityDays * 86_400_000;
    const percent = mastered
      ? 100
      : Math.min(
          92,
          Math.round(
            Math.max(observations * 9, stabilityDays * 4, current ? 12 : 0),
          ),
        );
    return {
      kcId: entry.kc.id,
      title: content.title(entry, locale),
      mastered,
      observations,
      stabilityDays,
      percent,
      lastReview,
      due,
    };
  });
}

export function totalReviews(progress: readonly KcProgress[]): number {
  return progress.reduce((sum, kc) => sum + kc.observations, 0);
}

export function dueCount(progress: readonly KcProgress[]): number {
  return progress.filter((kc) => kc.due).length;
}

const FSRS_DECAY = -0.5;
const FSRS_FACTOR = Math.pow(0.9, 1 / FSRS_DECAY) - 1;

export function retrievability(
  stabilityDays: number,
  lastReview: number,
  now = Date.now(),
): number {
  if (stabilityDays <= 0) {
    return 0;
  }
  const elapsedDays = Math.max(0, (now - lastReview) / 86_400_000);
  return Math.pow(1 + (FSRS_FACTOR * elapsedDays) / stabilityDays, FSRS_DECAY);
}

export function memoryRetention(
  progress: readonly KcProgress[],
  now = Date.now(),
): number {
  const cards = progress.filter(
    (kc) => kc.stabilityDays > 0 && kc.lastReview !== null,
  );
  if (cards.length === 0) {
    return 0;
  }
  const total = cards.reduce(
    (sum, kc) =>
      sum +
      retrievability(kc.stabilityDays, kc.lastReview ?? now, now),
    0,
  );
  return Math.round((total / cards.length) * 100);
}

export function weakestKcs(
  progress: readonly KcProgress[],
  limit = 3,
): KcProgress[] {
  return [...progress]
    .filter((kc) => !kc.mastered && kc.observations > 0)
    .sort((left, right) => left.percent - right.percent)
    .slice(0, Math.max(0, limit));
}

export function statsSummary(progress: KcProgress[]): StatsSummary {
  const withMemory = progress.filter((kc) => kc.stabilityDays > 0);
  return {
    mastered: progress.filter((kc) => kc.mastered).length,
    inMemory: withMemory.length,
    averageStability:
      withMemory.length === 0
        ? 0
        : withMemory.reduce((sum, kc) => sum + kc.stabilityDays, 0) /
          withMemory.length,
    calibration: null,
  };
}
