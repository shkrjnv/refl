// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";

import { BrandLogo, CrossfadeIcon, StudioIcon } from "./StudioIcons";

afterEach(cleanup);

describe("studio identity", () => {
  it("renders the pure-vector reflection arcs without raster or gradients", () => {
    const { container } = render(<BrandLogo size={36} />);
    const svg = container.querySelector("svg.brand-logo");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 28 28");
    expect(svg?.getAttribute("height")).toBe("36");
    expect(svg?.getAttribute("width")).toBe("36");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(svg?.getAttribute("data-mark")).toBe("reflection");
    expect(container.querySelector("image")).toBeNull();
    expect(container.querySelector("defs")).toBeNull();
    expect(container.querySelectorAll("linearGradient")).toHaveLength(0);
    const paths = container.querySelectorAll("path");
    expect(paths).toHaveLength(2);
    expect(paths[0]?.getAttribute("stroke")).toBe("currentColor");
    expect(paths[1]?.getAttribute("stroke")).toBe("var(--accent-primary)");
    expect(paths[0]?.getAttribute("fill")).toBeNull();
    expect(paths[1]?.getAttribute("fill")).toBeNull();

    const ink = paths[0]?.getAttribute("d") ?? "";
    const accent = paths[1]?.getAttribute("d") ?? "";
    const inkEnd = ink.match(/L([\d.]+) ([\d.]+)$/);
    const inkStart = ink.match(/^M([\d.]+) ([\d.]+)/);
    const accentStart = accent.match(/^M([\d.]+) ([\d.]+)/);
    const accentEnd = accent.match(/L([\d.]+) ([\d.]+)$/);
    expect(inkEnd).not.toBeNull();
    expect(inkStart).not.toBeNull();
    expect(accentStart).not.toBeNull();
    expect(accentEnd).not.toBeNull();
    if (!inkEnd || !inkStart || !accentStart || !accentEnd) {
      return;
    }
    const strokeWidth = Number(paths[0]?.getAttribute("stroke-width"));
    const topGap =
      Math.hypot(
        Number(accentStart[1]) - Number(inkEnd[1]),
        Number(accentStart[2]) - Number(inkEnd[2]),
      ) - strokeWidth;
    const bottomGap =
      Math.hypot(
        Number(accentEnd[1]) - Number(inkStart[1]),
        Number(accentEnd[2]) - Number(inkStart[2]),
      ) - strokeWidth;
    expect(topGap).toBeGreaterThanOrEqual(2.5);
    expect(topGap).toBeLessThanOrEqual(3.1);
    expect(bottomGap).toBeGreaterThanOrEqual(2.5);
    expect(bottomGap).toBeLessThanOrEqual(3.1);
    for (const size of [24, 32, 48]) {
      const { container: sized, unmount } = render(<BrandLogo size={size} />);
      const mark = sized.querySelector("svg.brand-logo");
      expect(mark?.getAttribute("width")).toBe(String(size));
      expect(mark?.getAttribute("height")).toBe(String(size));
      unmount();
    }
  });

  it("crossfades icon states with opacity, scale, and blur", () => {
    const { container, rerender } = render(
      <CrossfadeIcon name="plus" activeName="minus" active={false} size={18} />,
    );
    const wrapper = container.querySelector(".crossfade-icon");
    expect(wrapper?.getAttribute("data-active")).toBe("false");
    expect(wrapper?.getAttribute("style")).toContain("width: 18px");
    expect(container.querySelectorAll(".crossfade-icon-state")).toHaveLength(2);
    expect(container.querySelectorAll(".studio-icon")).toHaveLength(2);

    rerender(
      <CrossfadeIcon name="plus" activeName="minus" active size={18} />,
    );
    expect(
      container.querySelector(".crossfade-icon")?.getAttribute("data-active"),
    ).toBe("true");

    const css = readFileSync("src/app.css", "utf8");
    const crossfade = css.slice(css.lastIndexOf(".crossfade-icon-state {"));
    expect(crossfade).toContain("opacity 140ms ease");
    expect(crossfade).not.toContain("filter: blur(4px)");
    expect(crossfade).toContain("scale(0.94)");
    expect(css).toMatch(
      /prefers-reduced-motion: reduce[\s\S]*?\.crossfade-icon-state/,
    );
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
