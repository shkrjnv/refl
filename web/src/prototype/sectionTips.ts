const TIP_KEY = "refl.tip.";

export const SECTION_TIPS = [
  "vocabulary",
  "library",
  "graph",
  "notes",
] as const;

export type SectionTipId = (typeof SECTION_TIPS)[number];

function storage(): Storage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

/** A section tip is shown once per device until the learner resets them. */
export function shouldShowSectionTip(section: SectionTipId): boolean {
  const store = storage();
  if (!store) {
    return true;
  }
  try {
    return store.getItem(`${TIP_KEY}${section}`) !== "seen";
  } catch {
    return true;
  }
}

export function markSectionTipSeen(section: SectionTipId): void {
  const store = storage();
  if (!store) {
    return;
  }
  try {
    store.setItem(`${TIP_KEY}${section}`, "seen");
  } catch {
    // The tip can reappear if storage is blocked; the UI still works.
  }
}

export function resetSectionTips(): void {
  const store = storage();
  if (!store) {
    return;
  }
  try {
    for (const section of SECTION_TIPS) {
      store.removeItem(`${TIP_KEY}${section}`);
    }
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}
