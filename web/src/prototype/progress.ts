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
): KcProgress[] {
  return content.visibleArticles(locale).map((entry) => {
    const current = state.kcs[entry.kc.id];
    const mastered = current?.ever_mastered === true;
    const observations = current?.observations ?? 0;
    const stabilityDays = current?.card?.stability ?? 0;
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
    };
  });
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
