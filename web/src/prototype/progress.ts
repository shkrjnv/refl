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

export interface MisconceptionStateView {
  alpha: number;
  beta: number;
  manifested: number;
  updated_at: number | null;
}

export interface LearnerState {
  kcs: Record<string, KcStateView>;
  misconceptions: Record<string, MisconceptionStateView>;
  /** `profile.start_level.en`, absent until entry or placement records it. */
  startLevel: number | null;
}

export interface KcProgress {
  kcId: string;
  title: string;
  mastered: boolean;
  observations: number;
  stabilityDays: number;
  difficulty: number;
  percent: number;
  lastReview: number | null;
  due: boolean;
  level: string;
  domain: string;
}

export interface StatsSummary {
  mastered: number;
  inMemory: number;
  averageStability: number;
  averageDifficulty: number;
  calibration: number | null;
}

export interface LevelBreakdown {
  level: string;
  total: number;
  mastered: number;
  percent: number;
}

export interface DomainBreakdown {
  domain: string;
  total: number;
  mastered: number;
  percent: number;
}

export type MisconceptionStatus = "active" | "cured" | "open";

export interface MisconceptionProgress {
  id: string;
  title: string;
  strength: number;
  manifested: number;
  status: MisconceptionStatus;
}

function emptyLearnerState(): LearnerState {
  return { kcs: {}, misconceptions: {}, startLevel: null };
}

export function parseLearnerState(stateJson: string | null): LearnerState {
  if (!stateJson) {
    return emptyLearnerState();
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return emptyLearnerState();
  }
  if (!isJsonObject(parsed)) {
    return emptyLearnerState();
  }
  const profile = parsed["profile"];
  const startLevels = isJsonObject(profile) ? profile["start_level"] : null;
  const start = isJsonObject(startLevels) ? startLevels["en"] : null;
  const startLevel =
    typeof start === "number" && Number.isFinite(start) ? start : null;
  const kcs: Record<string, KcStateView> = {};
  const misconceptions: Record<string, MisconceptionStateView> = {};
  if (isJsonObject(parsed["misconceptions"])) {
    for (const [id, value] of Object.entries(parsed["misconceptions"])) {
      if (!isJsonObject(value)) {
        continue;
      }
      const alpha = value["alpha"];
      const beta = value["beta"];
      const manifested = value["manifested"];
      const updated = value["updated_at"];
      if (
        typeof alpha === "number" &&
        Number.isFinite(alpha) &&
        typeof beta === "number" &&
        Number.isFinite(beta)
      ) {
        misconceptions[id] = {
          alpha,
          beta,
          manifested:
            typeof manifested === "number" && Number.isFinite(manifested)
              ? manifested
              : 0,
          updated_at:
            typeof updated === "number" && Number.isFinite(updated)
              ? updated
              : null,
        };
      }
    }
  }
  if (!isJsonObject(parsed["kcs"])) {
    return { kcs: {}, misconceptions, startLevel };
  }
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
  return { kcs, misconceptions, startLevel };
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
    const difficulty = current?.card?.difficulty ?? 0;
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
      difficulty,
      percent,
      lastReview,
      due,
      level: entry.kc.level,
      domain: entry.kc.domain,
    };
  });
}

export function totalReviews(progress: readonly KcProgress[]): number {
  return progress.reduce((sum, kc) => sum + kc.observations, 0);
}

export function dueCount(progress: readonly KcProgress[]): number {
  return progress.filter((kc) => kc.due).length;
}

export function retrievability(
  stabilityDays: number,
  lastReview: number,
  now = Date.now(),
): number {
  if (stabilityDays <= 0) {
    return 0;
  }
  const elapsedDays = Math.max(0, (now - lastReview) / 86_400_000);
  return 1 / (1 + elapsedDays / (9 * stabilityDays));
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
    averageDifficulty:
      withMemory.length === 0
        ? 0
        : withMemory.reduce((sum, kc) => sum + kc.difficulty, 0) /
          withMemory.length,
    calibration: null,
  };
}

export function averageElapsedDays(
  progress: readonly KcProgress[],
  now = Date.now(),
): number {
  const reviewed = progress.filter(
    (kc) => kc.lastReview !== null && kc.stabilityDays > 0,
  );
  if (reviewed.length === 0) {
    return 0;
  }
  return (
    reviewed.reduce(
      (sum, kc) =>
        sum + Math.max(0, (now - (kc.lastReview ?? now)) / 86_400_000),
      0,
    ) / reviewed.length
  );
}

export function levelBreakdown(
  progress: readonly KcProgress[],
): LevelBreakdown[] {
  const groups = new Map<
    string,
    { total: number; mastered: number; percent: number }
  >();
  for (const kc of progress) {
    const current = groups.get(kc.level) ?? {
      total: 0,
      mastered: 0,
      percent: 0,
    };
    current.total += 1;
    current.mastered += kc.mastered ? 1 : 0;
    current.percent += kc.percent;
    groups.set(kc.level, current);
  }
  return [...groups.entries()]
    .map(([level, value]) => ({
      level,
      total: value.total,
      mastered: value.mastered,
      percent:
        value.total === 0 ? 0 : Math.round(value.percent / value.total),
    }))
    .sort((left, right) => left.level.localeCompare(right.level));
}

/** Curriculum bands shown in the level cards; Pre-A1 is a position label. */
export const CEFR_ORDER = ["A1", "A2", "B1", "B2"] as const;

/** Scale labels for a continuous position, pre-A1 = 0 … C2 = 6. */
export const CEFR_LABELS = [
  "Pre-A1",
  "A1",
  "A2",
  "B1",
  "B2",
  "C1",
  "C2",
] as const;

export interface LevelView {
  current: string;
  next: string;
  percent: number;
  mastered: number;
  total: number;
  preA1: boolean;
}

/**
 * The honest level card: a placement-derived position when one exists, a
 * Pre-A1 entry when there is no evidence, and the curriculum-coverage view
 * for a learner with answers but no placement.
 */
export function levelView(
  progress: readonly KcProgress[],
  startLevel: number | null,
): LevelView {
  const levels = levelBreakdown(progress);
  if (startLevel !== null) {
    const clamped = Math.max(
      0,
      Math.min(CEFR_LABELS.length - 1, startLevel),
    );
    const index = Math.floor(clamped);
    const current = CEFR_LABELS[index] ?? "Pre-A1";
    const next = CEFR_LABELS[Math.min(index + 1, CEFR_LABELS.length - 1)] ?? current;
    const entry = levels.find((candidate) => candidate.level === current);
    const total = entry?.total ?? 0;
    return {
      current,
      next,
      percent:
        total > 0
          ? (entry?.percent ?? 0)
          : Math.round((clamped - index) * 100),
      mastered: entry?.mastered ?? 0,
      total,
      preA1: current === "Pre-A1",
    };
  }
  if (totalReviews(progress) === 0) {
    return {
      current: "Pre-A1",
      next: "A1",
      percent: 0,
      mastered: 0,
      total: 0,
      preA1: true,
    };
  }
  const firstIncomplete = CEFR_ORDER.findIndex((level) => {
    const entry = levels.find((candidate) => candidate.level === level);
    return entry !== undefined && entry.percent < 100;
  });
  const highestObserved = CEFR_ORDER.reduce(
    (index, level, candidate) =>
      levels.some((entry) => entry.level === level && entry.total > 0)
        ? candidate
        : index,
    0,
  );
  const currentIndex = firstIncomplete >= 0 ? firstIncomplete : highestObserved;
  const current = CEFR_ORDER[currentIndex] ?? "A1";
  const next = CEFR_ORDER[currentIndex + 1] ?? current;
  const entry = levels.find((candidate) => candidate.level === current);
  return {
    current,
    next,
    percent: entry?.percent ?? 0,
    mastered: entry?.mastered ?? 0,
    total: entry?.total ?? 0,
    preA1: false,
  };
}

export function domainBreakdown(
  progress: readonly KcProgress[],
): DomainBreakdown[] {
  const groups = new Map<
    string,
    { total: number; mastered: number; percent: number }
  >();
  for (const kc of progress) {
    const current = groups.get(kc.domain) ?? {
      total: 0,
      mastered: 0,
      percent: 0,
    };
    current.total += 1;
    current.mastered += kc.mastered ? 1 : 0;
    current.percent += kc.percent;
    groups.set(kc.domain, current);
  }
  return [...groups.entries()]
    .map(([domain, value]) => ({
      domain,
      total: value.total,
      mastered: value.mastered,
      percent:
        value.total === 0 ? 0 : Math.round(value.percent / value.total),
    }))
    .sort((left, right) => right.total - left.total);
}

export function misconceptionProgress(
  content: PrototypeContent,
  state: LearnerState,
  locale: Locale,
): MisconceptionProgress[] {
  return Object.entries(state.misconceptions)
    .map(([id, value]) => {
      const evidence = value.alpha + value.beta;
      const strength = evidence > 0 ? value.alpha / evidence : 0;
      let status: MisconceptionStatus = "open";
      if (strength > 0.35 && value.manifested >= 2) {
        status = "active";
      } else if (strength < 0.15 && evidence >= 8) {
        status = "cured";
      }
      return {
        id,
        title: content.misconceptionTitle(id, locale),
        strength,
        manifested: value.manifested,
        status,
      };
    })
    .sort((left, right) => right.strength - left.strength);
}

export function misconceptionCureRate(
  misconceptions: readonly MisconceptionProgress[],
): number | null {
  const active = misconceptions.filter((entry) => entry.status === "active");
  const cured = misconceptions.filter((entry) => entry.status === "cured");
  if (active.length + cured.length === 0) {
    return null;
  }
  return Math.round((cured.length / (active.length + cured.length)) * 100);
}
