// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { IellaSprite } from "./IellaSprite";

afterEach(cleanup);

describe("Iella sprite", () => {
  it("keeps a padded viewBox and articulated layers", () => {
    const { container } = render(<IellaSprite size={64} />);
    const svg = container.querySelector("svg");
    expect(svg?.getAttribute("viewBox")).toBe("-1 -2 17 16");
    expect(svg?.getAttribute("overflow")).toBe("visible");
    expect(container.querySelectorAll(".iella-wing")).toHaveLength(2);
    expect(container.querySelector(".iella-eyes")).toBeTruthy();
    expect(container.querySelector(".iella-feet")).toBeTruthy();
    const height = Number(svg?.getAttribute("height"));
    expect(height).toBeCloseTo((64 * 16) / 17, 4);
  });

  it("exposes mood and perch state for the companion engine", () => {
    const { container, rerender } = render(
      <IellaSprite size={64} mood="perched" look="left" />,
    );
    let svg = container.querySelector("svg");
    expect(svg?.getAttribute("data-mood")).toBe("perched");
    expect(svg?.getAttribute("data-perched")).toBe("true");
    expect(svg?.getAttribute("data-look")).toBe("left");

    rerender(<IellaSprite size={64} mood="flying" />);
    svg = container.querySelector("svg");
    expect(svg?.getAttribute("data-mood")).toBe("flying");
    expect(svg?.getAttribute("data-perched")).toBe("false");

    rerender(<IellaSprite size={64} mood="proud" />);
    expect(container.querySelector(".iella-happy-eyes")).toBeTruthy();
  });

  it("exposes an optional labelled tip button", () => {
    let opened = false;
    render(
      <IellaSprite
        size={48}
        mood="curious"
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
