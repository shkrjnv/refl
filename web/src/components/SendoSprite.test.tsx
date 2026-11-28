// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { SendoSprite, type SendoMood } from "./SendoSprite";

afterEach(cleanup);

describe("Sendo pixel sprite", () => {
  it("keeps whole-number scaling on the 16 px grid", () => {
    const { container } = render(<SendoSprite size={64} decorative={false} label="Sendo" />);
    const svg = container.querySelector("svg");
    expect(svg?.getAttribute("width")).toBe("64");
    expect(svg?.getAttribute("height")).toBe("64");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 16 16");
    expect(svg?.getAttribute("role")).toBe("img");
  });

  it("exposes every emotion state without emoji", () => {
    const moods: SendoMood[] = [
      "idle",
      "happy",
      "celebrate",
      "thinking",
      "sleep",
      "surprised",
    ];
    for (const mood of moods) {
      const { container, unmount } = render(<SendoSprite mood={mood} />);
      expect(container.querySelector("svg")?.getAttribute("data-mood")).toBe(mood);
      expect(container.textContent).toBe("");
      unmount();
    }
  });
});
