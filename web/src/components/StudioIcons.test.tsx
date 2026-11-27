// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { BrandLogo, StudioIcon } from "./StudioIcons";

afterEach(cleanup);

describe("studio identity", () => {
  it("renders the crane-fused brand emblem without a raster import", () => {
    const { container } = render(<BrandLogo size={36} />);
    const svg = container.querySelector("svg.brand-logo");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 64 64");
    expect(svg?.getAttribute("height")).toBe("36");
    expect(svg?.getAttribute("width")).toBe("36");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector("image")).toBeNull();
    expect(container.querySelectorAll("defs linearGradient")).toHaveLength(6);
    expect(container.querySelectorAll(".brand-crane-body")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-crane-feather-umber")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-crane-feather-burgundy")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-crane-feather-cream")).toHaveLength(2);
    expect(container.querySelectorAll(".brand-crane-tail")).toHaveLength(2);
    expect(container.querySelectorAll(".brand-crane-neck")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-crane-head")).toHaveLength(1);
    expect(container.querySelectorAll(".brand-crane-beak")).toHaveLength(1);
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
