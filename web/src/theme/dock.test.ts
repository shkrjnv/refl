import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("../app.css", import.meta.url), "utf8");

describe("frosted bottom dock CSS", () => {
  it("keeps the dock translucent, frosted, and clear of the scroll content", () => {
    expect(css).toMatch(
      /data-nav-position="bottom"[\s\S]*?\.studio-content\s*\{[^}]*padding-bottom:\s*96px/,
    );
    expect(css).toMatch(/backdrop-filter:\s*blur\(18px\)\s*saturate\(165%\)/);
    expect(css).toMatch(/inset 0 1px 0/);
  });
});
