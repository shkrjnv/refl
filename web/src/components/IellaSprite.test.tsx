// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { IellaSprite } from "./IellaSprite";

afterEach(cleanup);

describe("Iella sprite", () => {
  it("keeps a padded viewBox and matching aspect ratio", () => {
    const { container } = render(<IellaSprite size={64} />);
    const svg = container.querySelector("svg");
    expect(svg?.getAttribute("viewBox")).toBe("-1 -2 16 15");
    expect(svg?.getAttribute("height")).toBe("60");
    expect(svg?.getAttribute("overflow")).toBe("visible");
  });

  it("exposes an optional labelled tip button", () => {
    let opened = false;
    render(
      <IellaSprite
        size={48}
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
