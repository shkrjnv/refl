import { readFileSync } from "node:fs";

import * as fontkit from "fontkit";
import { describe, expect, it } from "vitest";

function asset(path: string): string {
  return new URL(`../../public/${path}`, import.meta.url).pathname;
}

function font(path: string) {
  return fontkit.openSync(asset(path));
}

function covers(fontFile: ReturnType<typeof font>, text: string): string[] {
  return [...text].filter(
    (character) =>
      !fontFile.hasGlyphForCodePoint(character.codePointAt(0) ?? 0),
  );
}

const LATIN =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const CYRILLIC =
  "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюя";
const UZBEK = "ʻ’";
const IPA = "əθðŋʃʒɪʊæɜɒʌːˈˌɡ";
const REQUIRED = `${LATIN}${CYRILLIC}${UZBEK}${IPA}`;

describe("self-hosted fonts", () => {
  it("covers Latin, Cyrillic, Uzbek and IPA without a system fallback", () => {
    const geologica = font("fonts/Geologica-Variable.woff2");
    const ipa = font("fonts/Geologica-IPA.woff2");
    const missing = REQUIRED.split("").filter(
      (character) =>
        !geologica.hasGlyphForCodePoint(character.codePointAt(0) ?? 0) &&
        !ipa.hasGlyphForCodePoint(character.codePointAt(0) ?? 0),
    );
    expect(missing).toEqual([]);
    expect(covers(ipa, UZBEK + IPA)).toEqual([]);
  });

  it("keeps the pixel accent face local and usable", () => {
    const pixel = font("fonts/PixelifySans-Variable.woff2");
    expect(pixel.familyName).toMatch(/Pixelify Sans/);
    expect(covers(pixel, "refl 0123")).toEqual([]);
  });

  it("declares only local font URLs", () => {
    const css = readFileSync(
      new URL("./fonts.css", import.meta.url),
      "utf8",
    );
    expect(css).not.toMatch(/https?:\/\//);
    expect(css.match(/url\("\/fonts\/[^"]+\.woff2"\)/g)).toHaveLength(3);
    expect(css).toContain('font-family: "Geologica"');
    expect(css).toContain('font-family: "Pixelify Sans"');
  });
});

