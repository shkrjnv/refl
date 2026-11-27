// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { BrandLogo, StudioIcon } from "./StudioIcons";

afterEach(cleanup);

describe("studio identity", () => {
  it("renders the two-bracket prism reflection mark without a raster import", () => {
    const { container } = render(<BrandLogo size={36} />);
    const svg = container.querySelector("svg.brand-logo");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 40 40");
    expect(svg?.getAttribute("height")).toBe("36");
    expect(svg?.getAttribute("width")).toBe("36");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(svg?.getAttribute("data-mark")).toBe("prism-reflection");
    expect(container.querySelector("image")).toBeNull();
    expect(container.querySelectorAll("defs linearGradient")).toHaveLength(5);
    expect(container.querySelectorAll(".brand-prism-stem")).toHaveLength(2);
    expect(container.querySelectorAll(".brand-prism-arm")).toHaveLength(2);
    expect(container.querySelectorAll(".brand-prism-stem-umber")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-prism-stem-blue")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-prism-arm-cream")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-prism-arm-teal")).toHaveLength(1);
    for (const size of [24, 32, 48]) {
      const { container: sized, unmount } = render(<BrandLogo size={size} />);
      const mark = sized.querySelector("svg.brand-logo");
      expect(mark?.getAttribute("width")).toBe(String(size));
      expect(mark?.getAttribute("height")).toBe(String(size));
      unmount();
    }
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
