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

export function parseLearnerState(stateJson: string | null): LearnerState {
  if (!stateJson) {
    return { kcs: {}, misconceptions: {} };
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(stateJson) as unknown;
  } catch {
    return { kcs: {}, misconceptions: {} };
  }
  if (!isJsonObject(parsed)) {
    return { kcs: {}, misconceptions: {} };
  }
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
    return { kcs: {}, misconceptions };
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
  return { kcs, misconceptions };
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
