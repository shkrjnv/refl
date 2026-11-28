import { describe, expect, it } from "vitest";

import {
  DEFAULT_READER_SETTINGS,
  parseReaderSettings,
  readerStyle,
} from "./readerSettings";

describe("reader settings", () => {
  it("reads valid settings and falls back per field", () => {
    expect(
      parseReaderSettings(
        JSON.stringify({
          theme: "midnight",
          size: "xl",
          lineHeight: "1.9",
          measure: "75",
          layout: "scroll",
        }),
      ),
    ).toEqual({
      theme: "midnight",
      size: "xl",
      lineHeight: "1.9",
      measure: "75",
      layout: "scroll",
    });
    expect(
      parseReaderSettings(
        JSON.stringify({
          theme: "neon",
          size: "xxl",
          lineHeight: "2",
          measure: "40",
          layout: "spread",
        }),
      ),
    ).toEqual(DEFAULT_READER_SETTINGS);
    expect(parseReaderSettings("not json")).toEqual(DEFAULT_READER_SETTINGS);
    expect(parseReaderSettings(null)).toEqual(DEFAULT_READER_SETTINGS);
  });

  it("opens books in paged mode unless the reader chose scroll", () => {
    expect(DEFAULT_READER_SETTINGS.layout).toBe("paged");
    // Settings saved before the layout existed keep their typography.
    expect(
      parseReaderSettings(JSON.stringify({ theme: "sepia", size: "l" })),
    ).toMatchObject({ theme: "sepia", size: "l", layout: "paged" });
  });

  it("maps typography to CSS variables", () => {
    expect(
      readerStyle({
        theme: "sepia",
        size: "l",
        lineHeight: "1.5",
        measure: "55",
        layout: "paged",
      }),
    ).toEqual({
      "--reader-font-size": "20px",
      "--reader-line-height": "1.5",
      "--reader-measure": "55ch",
    });
  });
});
