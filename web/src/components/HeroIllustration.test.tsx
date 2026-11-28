// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { HeroIllustration } from "./HeroIllustration";

afterEach(cleanup);

describe("Today hero illustration", () => {
  it("offers AVIF and WebP candidates before the JPEG fallback", () => {
    const { container } = render(<HeroIllustration />);
    const picture = container.querySelector("picture");
    const sources = [...container.querySelectorAll("source")];
    const image = container.querySelector("img");
    expect(picture).toBeTruthy();
    expect(image).toBeTruthy();
    expect(sources.map((source) => source.type)).toEqual([
      "image/avif",
      "image/webp",
    ]);
    for (const [index, type] of ["avif", "webp"].entries()) {
      const source = sources[index];
      expect(source?.getAttribute("srcset")).toContain(`.${type} 480w`);
      expect(source?.getAttribute("srcset")).toContain(`.${type} 2048w`);
      expect(source?.getAttribute("sizes")).toContain("1180px");
    }
    expect(image?.getAttribute("src")).toContain("hero-nature-1120.jpg");
    expect(image?.getAttribute("srcset")).toContain("hero-nature-2048.jpg 2048w");
    expect(image?.getAttribute("sizes")).toContain("1180px");
    expect(image?.getAttribute("width")).toBe("3000");
    expect(image?.getAttribute("height")).toBe("1792");
    expect(image?.getAttribute("loading")).toBe("eager");
    expect(image?.getAttribute("fetchpriority")).toBe("high");
    expect(image?.getAttribute("decoding")).toBe("async");
  });
});
