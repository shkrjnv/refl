// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { IellaSprite } from "./IellaSprite";

afterEach(cleanup);

describe("Iella sprite", () => {
  it("uses the canonical 12 x 11 artboard and integer pixel scaling", () => {
    const { container } = render(<IellaSprite size={64} />);
    const svg = container.querySelector("svg");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 12 11");
    expect(svg?.getAttribute("shape-rendering")).toBe("crispEdges");
    expect(svg?.getAttribute("overflow")).toBe("hidden");
    expect(svg?.getAttribute("width")).toBe("60");
    expect(svg?.getAttribute("height")).toBe("55");
    expect(Number(svg?.getAttribute("width")) % 12).toBe(0);
    expect(Number(svg?.getAttribute("height")) % 11).toBe(0);
  });

  it("keeps every canonical pixel inside the artboard", () => {
    const { container } = render(<IellaSprite size={72} />);
    const rects = container.querySelectorAll("rect");
    expect(rects).toHaveLength(28);
    for (const rect of rects) {
      const x = Number(rect.getAttribute("x"));
      const y = Number(rect.getAttribute("y"));
      const width = Number(rect.getAttribute("width"));
      const height = Number(rect.getAttribute("height"));
      expect(x + width).toBeLessThanOrEqual(12);
      expect(y + height).toBeLessThanOrEqual(11);
    }
    const eye = container.querySelector(".iella-eye-fill");
    expect(eye?.getAttribute("x")).toBe("6");
    expect(eye?.getAttribute("y")).toBe("2");
    expect(container.querySelectorAll(".iella-wing")).toHaveLength(1);
    expect(container.querySelector(".iella-feet")).toBeTruthy();
  });

  it("exposes calm mood and look state for the studio surfaces", () => {
    const { container, rerender } = render(
      <IellaSprite size={60} mood="perched" look="left" />,
    );
    let svg = container.querySelector("svg");
    expect(svg?.getAttribute("data-mood")).toBe("perched");
    expect(svg?.getAttribute("data-perched")).toBe("true");
    expect(svg?.getAttribute("data-look")).toBe("left");

    rerender(<IellaSprite size={60} mood="idle" />);
    svg = container.querySelector("svg");
    expect(svg?.getAttribute("data-mood")).toBe("idle");
    expect(svg?.getAttribute("data-perched")).toBe("false");
  });

  it("exposes an optional labelled tip button", () => {
    let opened = false;
    render(
      <IellaSprite
        size={48}
        mood="idle"
        label="Ask Iella for a tip"
        onActivate={() => {
          opened = true;
        }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Ask Iella for a tip" }));
    expect(opened).toBe(true);
  });
});
