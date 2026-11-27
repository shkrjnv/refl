// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { BrandLogo, StudioIcon } from "./StudioIcons";

afterEach(cleanup);

describe("studio identity", () => {
  it("renders the dimensional 3D-ribbon brand emblem without a raster import", () => {
    const { container } = render(<BrandLogo size={36} />);
    const svg = container.querySelector("svg.brand-logo");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 52 62");
    expect(svg?.getAttribute("height")).toBe("36");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector("image")).toBeNull();
    expect(container.querySelectorAll("defs linearGradient")).toHaveLength(6);
    expect(container.querySelectorAll(".brand-ribbon-cream")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-ribbon-dark")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-ribbon-blue")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-ribbon-teal")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-ribbon-cream-highlight")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-ribbon-blue-dark")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-ribbon-teal-highlight")).toHaveLength(1);
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
