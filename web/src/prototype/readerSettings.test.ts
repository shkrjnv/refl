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
        }),
      ),
    ).toEqual({
      theme: "midnight",
      size: "xl",
      lineHeight: "1.9",
      measure: "75",
    });
    expect(
      parseReaderSettings(
        JSON.stringify({
          theme: "neon",
          size: "xxl",
          lineHeight: "2",
          measure: "40",
        }),
      ),
    ).toEqual(DEFAULT_READER_SETTINGS);
    expect(parseReaderSettings("not json")).toEqual(DEFAULT_READER_SETTINGS);
    expect(parseReaderSettings(null)).toEqual(DEFAULT_READER_SETTINGS);
  });

  it("maps typography to CSS variables", () => {
    expect(
      readerStyle({
        theme: "sepia",
        size: "l",
        lineHeight: "1.5",
        measure: "55",
      }),
    ).toEqual({
      "--reader-font-size": "20px",
      "--reader-line-height": "1.5",
      "--reader-measure": "55ch",
    });
  });
});
