import type { ActiveContentResponse } from "../api/client";
import type { Explanation } from "../generated/core/Explanation";
import type { Instruction } from "../generated/core/Instruction";
import type { Item } from "../generated/core/Item";
import type { Kc } from "../generated/core/Kc";
import type { LangReviews } from "../generated/core/LangReviews";
import type { Localized } from "../generated/core/Localized";
import type { Slot } from "../generated/core/Slot";
import type { Locale } from "../i18n";
import {
  isJsonObject,
  type JsonObject,
  type JsonValue,
} from "../types/json";

export interface KcEntry {
  kc: Kc;
  status: "live" | "draft";
  level_rank: number;
}

export interface ItemEntry {
  item: Item;
  session_eligible: boolean;
}

function asArray(value: JsonValue | undefined): JsonValue[] {
  return Array.isArray(value) ? value : [];
}

function asString(value: JsonValue | undefined, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function parseKcs(value: JsonValue | undefined): KcEntry[] {
  return asArray(value).flatMap((entry) => {
    if (!isJsonObject(entry) || !isJsonObject(entry["kc"])) {
      return [];
    }
    return [
      {
        kc: entry["kc"] as unknown as Kc,
        status: entry["status"] === "live" ? "live" : "draft",
        level_rank:
          typeof entry["level_rank"] === "number"
            ? entry["level_rank"]
            : 0,
      },
    ];
  });
}

function parseItems(value: JsonValue | undefined): ItemEntry[] {
  return asArray(value).flatMap((entry) => {
    if (!isJsonObject(entry) || !isJsonObject(entry["item"])) {
      return [];
    }
    return [
      {
        item: entry["item"] as unknown as Item,
        session_eligible: entry["session_eligible"] === true,
      },
    ];
  });
}

function parseObjects<T>(value: JsonValue | undefined): T[] {
  return asArray(value).filter(isJsonObject) as unknown as T[];
}

function itemSlots(item: Item): Slot[] {
  const slots = (item as unknown as { slots?: Slot[] }).slots;
  return Array.isArray(slots) ? slots : [];
}

function reviewed(reviews: LangReviews | undefined, locale: Locale): boolean {
  return reviews?.[locale]?.status === "reviewed";
}

export function localizedText(
  text: Localized | undefined,
  locale: Locale,
): string {
  if (!text) {
    return "";
  }
  return text[locale] ?? text.en ?? text.ru ?? text.uz ?? "";
}

function collectWords(value: JsonValue, words: Set<string>): void {
  if (typeof value === "string") {
    const matches = value.match(/[A-Za-z]+(?:['’][A-Za-z]+)?/g) ?? [];
    for (const match of matches) {
      words.add(match.toLowerCase());
    }
    return;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      collectWords(item, words);
    }
    return;
  }
  if (isJsonObject(value)) {
    for (const item of Object.values(value)) {
      collectWords(item, words);
    }
  }
}

export function lexiconTextForBundle(bundle: JsonValue): string {
  const words = new Set<string>();
  collectWords(bundle, words);
  const sorted = [...words].sort();
  return sorted.length === 0 ? "" : `${sorted.join("\n")}\n`;
}

export class PrototypeContent {
  readonly bundleJson: string;
  readonly contentVersion: string;
  readonly digest: string;
  readonly kcs: KcEntry[];
  readonly items: ItemEntry[];
  readonly explanations: Explanation[];
  readonly instructions: Instruction[];
  readonly lexiconText: string;
  readonly lexiconJson: string;

  constructor(
    bundleJson: string,
    contentVersion: string,
    digest: string,
    lexiconText?: string,
  ) {
    const parsed: unknown = JSON.parse(bundleJson);
    if (!isJsonObject(parsed)) {
      throw new Error("content bundle must be a JSON object");
    }
    const content = parsed["content"];
    if (!isJsonObject(content)) {
      throw new Error("content bundle has no content object");
    }
    this.bundleJson = bundleJson;
    this.contentVersion =
      contentVersion || asString(parsed["content_version"], "unknown");
    this.digest = digest;
    this.kcs = parseKcs(content["kcs"]);
    this.items = parseItems(content["items"]);
    this.explanations = parseObjects<Explanation>(content["explanations"]);
    this.instructions = parseObjects<Instruction>(content["instructions"]);
    this.lexiconText = lexiconText ?? lexiconTextForBundle(parsed);
    this.lexiconJson = JSON.stringify({ text: this.lexiconText });
  }

  static fromActive(response: ActiveContentResponse): PrototypeContent {
    return new PrototypeContent(
      JSON.stringify(response.bundle),
      response.content_version,
      response.digest,
    );
  }

  articles(): KcEntry[] {
    return this.kcs.filter(
      (entry) =>
        entry.kc.id.startsWith("en.gram.") &&
        entry.kc.id !== "en.orth.spelling" &&
        entry.status === "live" &&
        !entry.kc.observed_only,
    );
  }

  visibleArticles(locale: Locale): KcEntry[] {
    return this.articles().filter((entry) =>
      reviewed(entry.kc.review, locale),
    );
  }

  title(kc: KcEntry, locale: Locale): string {
    return localizedText(kc.kc.title, locale);
  }

  kcTitle(kcId: string, locale: Locale): string {
    const entry = this.kcs.find((candidate) => candidate.kc.id === kcId);
    return entry ? this.title(entry, locale) : kcId;
  }

  itemsForKc(kcId: string, locale: Locale): ItemEntry[] {
    return this.items.filter(
      (entry) =>
        entry.session_eligible &&
        entry.item.status === "live" &&
        reviewed(entry.item.review, locale) &&
        itemSlots(entry.item).some((slot) =>
          slot.kcs.some((row) => row.kc === kcId),
        ),
    );
  }

  item(ref: { id: string; version: number }): ItemEntry | null {
    return (
      this.items.find(
        (entry) =>
          entry.item.id === ref.id && entry.item.version === ref.version,
      ) ?? null
    );
  }

  itemForKc(kcId: string, locale: Locale): ItemEntry | null {
    return this.itemsForKc(kcId, locale)[0] ?? null;
  }

  instruction(item: Item, locale: Locale): string {
    const instruction = this.instructions.find(
      (candidate) => candidate.id === item.instruction,
    );
    return localizedText(instruction?.text, locale);
  }

  explanationFor(kcId: string, locale: Locale): Explanation | null {
    return this.explanationForTarget("kc", kcId, locale);
  }

  explanation(id: string): Explanation | null {
    return this.explanations.find((entry) => entry.id === id) ?? null;
  }

  misconceptionExplanation(
    misconceptionId: string,
    locale: Locale,
  ): Explanation | null {
    return this.explanationForTarget("misconception", misconceptionId, locale);
  }

  private explanationForTarget(
    kind: "kc" | "misconception",
    id: string,
    locale: Locale,
  ): Explanation | null {
    const exact = this.explanations.find(
      (entry) =>
        entry.target.kind === kind &&
        entry.target.id === id &&
        entry.lang === locale,
    );
    if (exact) {
      return exact;
    }
    return (
      this.explanations.find(
        (entry) =>
          entry.target.kind === kind &&
          entry.target.id === id &&
          entry.lang === "en",
      ) ?? null
    );
  }
}

export type { JsonObject };
