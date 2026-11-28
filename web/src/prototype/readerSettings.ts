export type ReaderTheme = "paper" | "sepia" | "dusk" | "midnight";
export type ReaderSize = "s" | "m" | "l" | "xl";
export type ReaderLineHeight = "1.5" | "1.7" | "1.9";
export type ReaderMeasure = "55" | "65" | "75";

export interface ReaderSettings {
  theme: ReaderTheme;
  size: ReaderSize;
  lineHeight: ReaderLineHeight;
  measure: ReaderMeasure;
}

export const READER_THEMES: readonly ReaderTheme[] = [
  "paper",
  "sepia",
  "dusk",
  "midnight",
];
export const READER_SIZES: readonly ReaderSize[] = ["s", "m", "l", "xl"];
export const READER_LINE_HEIGHTS: readonly ReaderLineHeight[] = [
  "1.5",
  "1.7",
  "1.9",
];
export const READER_MEASURES: readonly ReaderMeasure[] = ["55", "65", "75"];

export const DEFAULT_READER_SETTINGS: ReaderSettings = {
  theme: "paper",
  size: "m",
  lineHeight: "1.7",
  measure: "65",
};

const STORAGE_KEY = "refl.reader.v1";

const FONT_SIZES: Record<ReaderSize, string> = {
  s: "16px",
  m: "18px",
  l: "20px",
  xl: "23px",
};

function isTheme(value: unknown): value is ReaderTheme {
  return typeof value === "string" && READER_THEMES.includes(value as ReaderTheme);
}

function isSize(value: unknown): value is ReaderSize {
  return typeof value === "string" && READER_SIZES.includes(value as ReaderSize);
}

function isLineHeight(value: unknown): value is ReaderLineHeight {
  return (
    typeof value === "string" &&
    READER_LINE_HEIGHTS.includes(value as ReaderLineHeight)
  );
}

function isMeasure(value: unknown): value is ReaderMeasure {
  return (
    typeof value === "string" && READER_MEASURES.includes(value as ReaderMeasure)
  );
}

export function parseReaderSettings(raw: string | null): ReaderSettings {
  if (!raw) {
    return DEFAULT_READER_SETTINGS;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") {
      return DEFAULT_READER_SETTINGS;
    }
    const value = parsed as Record<string, unknown>;
    return {
      theme: isTheme(value["theme"]) ? value["theme"] : DEFAULT_READER_SETTINGS.theme,
      size: isSize(value["size"]) ? value["size"] : DEFAULT_READER_SETTINGS.size,
      lineHeight: isLineHeight(value["lineHeight"])
        ? value["lineHeight"]
        : DEFAULT_READER_SETTINGS.lineHeight,
      measure: isMeasure(value["measure"])
        ? value["measure"]
        : DEFAULT_READER_SETTINGS.measure,
    };
  } catch {
    return DEFAULT_READER_SETTINGS;
  }
}

export function readReaderSettings(): ReaderSettings {
  if (typeof localStorage === "undefined") {
    return DEFAULT_READER_SETTINGS;
  }
  try {
    return parseReaderSettings(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_READER_SETTINGS;
  }
}

export function writeReaderSettings(settings: ReaderSettings): void {
  if (typeof localStorage === "undefined") {
    return;
  }
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // A private window may deny storage; the current session still works.
  }
}

export function readerFontSize(size: ReaderSize): string {
  return FONT_SIZES[size];
}

export function readerStyle(settings: ReaderSettings): Record<string, string> {
  return {
    "--reader-font-size": readerFontSize(settings.size),
    "--reader-line-height": settings.lineHeight,
    "--reader-measure": `${settings.measure}ch`,
  };
}
