import type { Item } from "../generated/core/Item";
import type { ItemRef } from "../generated/core/ItemRef";
import type { Slot } from "../generated/core/Slot";
import type { Locale } from "../i18n";
import { localizedText, type PrototypeContent } from "./content";

export interface PresentedChoice {
  text: string;
  canonicalIndex: number;
}

export type ItemKind = "choice" | "find_error" | "gap_fill";

export interface ItemView {
  ref: ItemRef;
  item: Item;
  slot: number;
  kind: ItemKind;
  instruction: string;
  stimulus: string;
  choices: PresentedChoice[];
  tokens: string[];
  tokenChoice: number[];
  correctAnswer: string;
  why: string;
  hint: string;
  kcTitles: string[];
}

function hashText(text: string): number {
  let hash = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function shuffleChoices(
  choices: PresentedChoice[],
  seed: string,
): PresentedChoice[] {
  return choices
    .map((choice, index) => ({
      choice,
      key: hashText(`${seed}:${index}:${choice.text}`),
    }))
    .sort((left, right) => left.key - right.key)
    .map((entry) => entry.choice);
}

function itemSlots(item: Item): Slot[] {
  const slots = (item as unknown as { slots?: Slot[] }).slots;
  return Array.isArray(slots) ? slots : [];
}

function kindOf(item: Item): ItemKind {
  switch (item.type) {
    case "find_error":
    case "gap_fill":
      return item.type;
    default:
      return "choice";
  }
}

function cleanToken(token: string): string {
  return token.replace(/[^\p{L}']/gu, "").toLowerCase();
}

function buildTokenChoices(tokens: string[], choices: PresentedChoice[]): number[] {
  return tokens.map((token) => {
    const cleaned = cleanToken(token);
    if (cleaned.length === 0) {
      return -1;
    }
    const match = choices.find((choice) =>
      choice.text
        .split(/\s+/)
        .some((word) => cleanToken(word) === cleaned),
    );
    return match?.canonicalIndex ?? -1;
  });
}

export function buildItemView(
  content: PrototypeContent,
  item: Item,
  slotIndex: number,
  locale: Locale,
  seed: string,
): ItemView {
  const slot = itemSlots(item)[slotIndex];
  if (!slot) {
    throw new Error(`item ${item.id} has no slot ${slotIndex}`);
  }
  const kind = kindOf(item);
  const baseChoices: PresentedChoice[] =
    kind === "gap_fill"
      ? []
      : [
          { text: slot.answer, canonicalIndex: 0 },
          ...slot.options.map((option, index) => ({
            text: option.text,
            canonicalIndex: index + 1,
          })),
        ];
  const choices = shuffleChoices(baseChoices, `${item.id}:${item.version}:${seed}`);
  const stimulus = item.stimulus.text;
  const tokens =
    kind === "find_error"
      ? stimulus.split(/\s+/).filter((token) => token.length > 0)
      : [];
  return {
    ref: {
      id: item.id,
      version: item.version,
    },
    item,
    slot: slotIndex,
    kind,
    instruction: content.instruction(item, locale),
    stimulus,
    choices,
    tokens,
    tokenChoice:
      kind === "find_error" ? buildTokenChoices(tokens, choices) : [],
    correctAnswer: slot.answer,
    why: localizedText(slot.why, locale),
    hint: localizedText(slot.hint, locale),
    kcTitles: slot.kcs.map((row) => content.kcTitle(row.kc, locale)),
  };
}
