// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { BrandLogo, StudioIcon } from "./StudioIcons";

afterEach(cleanup);

describe("studio identity", () => {
  it("renders the folded-ribbon brand emblem", () => {
    const { container } = render(<BrandLogo size={28} />);
    const svg = container.querySelector("svg.brand-logo");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 28 28");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector(".brand-logo-field")?.getAttribute("rx")).toBe(
      "8.2",
    );
    expect(container.querySelectorAll(".brand-logo-fold")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-logo-reflect")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-logo-spine")).toHaveLength(1);
  });

  it("keeps the navigation icon set on the same stroke geometry", () => {
    for (const name of [
      "today",
      "practice",
      "progress",
      "profile",
      "settings",
      "gear",
    ] as const) {
      const { container, unmount } = render(<StudioIcon name={name} />);
      const svg = container.querySelector("svg.studio-icon");
      expect(svg?.getAttribute("viewBox")).toBe("0 0 24 24");
      expect(svg?.getAttribute("stroke-width")).toBe("1.75");
      unmount();
    }
  });
});
