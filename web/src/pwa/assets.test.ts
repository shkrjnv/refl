import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

function publicFile(path: string): Buffer {
  return readFileSync(new URL(`../../public/${path}`, import.meta.url));
}

function pngSize(bytes: Buffer): { width: number; height: number } {
  expect(bytes.subarray(0, 8)).toEqual(
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  );
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

describe("PWA assets", () => {
  it("declares local 192 and 512 icons and install colours", () => {
    const manifest = JSON.parse(
      publicFile("manifest.webmanifest").toString("utf8"),
    ) as {
      theme_color?: string;
      background_color?: string;
      icons?: Array<{
        src: string;
        sizes: string;
        type: string;
        purpose?: string;
      }>;
    };
    expect(manifest.theme_color).toBe("#0D7A78");
    expect(manifest.background_color).toBe("#FAF4E8");
    expect(manifest.icons).toHaveLength(2);
    for (const [icon, size] of [
      [manifest.icons?.[0], 192],
      [manifest.icons?.[1], 512],
    ] as const) {
      expect(icon?.src).toBe(`/icons/icon-${size}.png`);
      expect(icon?.sizes).toBe(`${size}x${size}`);
      expect(icon?.type).toBe("image/png");
      expect(icon?.purpose).toContain("maskable");
      expect(pngSize(publicFile(`icons/icon-${size}.png`))).toEqual({
        width: size,
        height: size,
      });
    }
    expect(JSON.stringify(manifest)).not.toMatch(/https?:\/\//);
  });

  it("links the manifest and theme colour from the document", () => {
    const html = readFileSync(new URL("../../index.html", import.meta.url), "utf8");
    expect(html).toContain('rel="manifest" href="/manifest.webmanifest"');
    expect(html).toContain('name="theme-color" content="#0D7A78"');
    expect(html).toContain('href="/icons/icon-192.png"');
  });
});
