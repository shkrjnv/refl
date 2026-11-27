import { useEffect, useRef } from "react";

type Rgb = readonly [number, number, number];

interface Blob {
  cx: number;
  cy: number;
  ax: number;
  ay: number;
  sx: number;
  sy: number;
  radius: number;
  phase: number;
  amplitude: number;
}

const BLOBS: readonly Blob[] = [
  { cx: 0.22, cy: 0.3, ax: 0.08, ay: 0.05, sx: 0.07, sy: 0.05, radius: 0.25, phase: 0.4, amplitude: 1.1 },
  { cx: 0.74, cy: 0.22, ax: 0.06, ay: 0.08, sx: 0.05, sy: 0.07, radius: 0.21, phase: 2.1, amplitude: 1.2 },
  { cx: 0.61, cy: 0.57, ax: 0.12, ay: 0.05, sx: 0.06, sy: 0.09, radius: 0.24, phase: 4.2, amplitude: 1.35 },
  { cx: 0.86, cy: 0.73, ax: 0.05, ay: 0.1, sx: 0.08, sy: 0.05, radius: 0.24, phase: 5.5, amplitude: 1.0 },
  { cx: 0.25, cy: 0.8, ax: 0.1, ay: 0.06, sx: 0.04, sy: 0.08, radius: 0.2, phase: 1.3, amplitude: 0.95 },
];

const CONTOURS = [0.22, 0.38, 0.56, 0.72] as const;

function cssColor(name: string): string {
  if (typeof window === "undefined") {
    return "";
  }
  return getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
}

function parseHex(value: string, fallback: Rgb): Rgb {
  const hex = value.replace("#", "");
  if (hex.length === 3) {
    const red = Number.parseInt(`${hex[0]}${hex[0]}`, 16);
    const green = Number.parseInt(`${hex[1]}${hex[1]}`, 16);
    const blue = Number.parseInt(`${hex[2]}${hex[2]}`, 16);
    return Number.isFinite(red) && Number.isFinite(green) && Number.isFinite(blue)
      ? [red, green, blue]
      : fallback;
  }
  if (hex.length === 6) {
    const red = Number.parseInt(hex.slice(0, 2), 16);
    const green = Number.parseInt(hex.slice(2, 4), 16);
    const blue = Number.parseInt(hex.slice(4, 6), 16);
    return Number.isFinite(red) && Number.isFinite(green) && Number.isFinite(blue)
      ? [red, green, blue]
      : fallback;
  }
  return fallback;
}

function mix(left: Rgb, right: Rgb, amount: number): Rgb {
  const t = Math.max(0, Math.min(1, amount));
  return [
    Math.round(left[0] + (right[0] - left[0]) * t),
    Math.round(left[1] + (right[1] - left[1]) * t),
    Math.round(left[2] + (right[2] - left[2]) * t),
  ];
}

function colorAt(value: number, ramp: readonly Rgb[], hot: Rgb): Rgb {
  if (value <= 0.18) {
    return mix(ramp[0] ?? hot, ramp[1] ?? hot, value / 0.18);
  }
  if (value <= 0.38) {
    return mix(ramp[1] ?? hot, ramp[2] ?? hot, (value - 0.18) / 0.2);
  }
  if (value <= 0.58) {
    return mix(ramp[2] ?? hot, ramp[3] ?? hot, (value - 0.38) / 0.2);
  }
  if (value <= 0.78) {
    return mix(ramp[3] ?? hot, ramp[4] ?? hot, (value - 0.58) / 0.2);
  }
  return mix(ramp[4] ?? hot, hot, (value - 0.78) / 0.22);
}

function fieldAt(x: number, y: number, time: number): number {
  let total = 0;
  for (const blob of BLOBS) {
    const cx =
      blob.cx +
      Math.sin(time * blob.sx + blob.phase) * blob.ax +
      Math.sin(time * blob.sx * 0.43 + blob.phase * 2.3) * blob.ax * 0.35;
    const cy =
      blob.cy +
      Math.cos(time * blob.sy + blob.phase) * blob.ay +
      Math.cos(time * blob.sy * 0.37 + blob.phase * 1.7) * blob.ay * 0.3;
    const radius =
      blob.radius * (0.92 + Math.sin(time * 0.05 + blob.phase) * 0.08);
    const dx = x - cx;
    const dy = y - cy;
    total +=
      blob.amplitude *
      Math.exp(-(dx * dx + dy * dy) / (2 * radius * radius));
  }
  return 1 - Math.exp(-total * 0.235);
}

function drawFrame(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  time: number,
  ramp: readonly Rgb[],
  hot: Rgb,
): void {
  const image = context.createImageData(width, height);
  const data = image.data;
  for (let y = 0; y < height; y += 1) {
    const ny = y / height;
    for (let x = 0; x < width; x += 1) {
      const nx = x / width;
      const value = fieldAt(nx, ny, time);
      let color = colorAt(value, ramp, hot);
      for (const contour of CONTOURS) {
        const distance = Math.abs(value - contour);
        if (distance < 0.017) {
          color = mix(color, hot, 0.5 * (1 - distance / 0.017));
        }
      }
      const offset = (y * width + x) * 4;
      data[offset] = color[0];
      data[offset + 1] = color[1];
      data[offset + 2] = color[2];
      data[offset + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
}

export function AuthBackdrop({ paused = false }: { paused?: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (
      !canvas ||
      typeof CanvasRenderingContext2D === "undefined" ||
      typeof canvas.getContext !== "function"
    ) {
      return;
    }
    const context = canvas.getContext("2d");
    if (!context) {
      return;
    }
    const base = parseHex(cssColor("--thermal-base"), [27, 11, 59]);
    const violet = parseHex(cssColor("--thermal-violet"), [63, 28, 120]);
    const crimson = parseHex(cssColor("--thermal-crimson"), [232, 74, 39]);
    const magenta = parseHex(cssColor("--thermal-magenta"), [217, 70, 239]);
    const amber = parseHex(cssColor("--thermal-amber"), [254, 240, 138]);
    const hot = parseHex(cssColor("--thermal-hot"), [255, 247, 198]);
    const ramp: readonly Rgb[] = [base, violet, crimson, magenta, amber];
    const reduced =
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let frame = 0;
    let disposed = false;
    let last = -1;

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const ratio = rect.width > 800 ? 0.58 : 0.8;
      const width = Math.max(180, Math.floor(Math.max(rect.width, 320) * ratio));
      const height = Math.max(180, Math.floor(Math.max(rect.height, 420) * ratio));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      drawFrame(context, width, height, reduced ? 4.2 : last < 0 ? 0 : last / 1000, ramp, hot);
    };

    resize();
    window.addEventListener("resize", resize);
    if (paused || reduced || typeof window.requestAnimationFrame !== "function") {
      return () => window.removeEventListener("resize", resize);
    }

    const tick = (now: number) => {
      if (disposed) {
        return;
      }
      if (now - last >= 45) {
        last = now;
        drawFrame(context, canvas.width, canvas.height, now / 1000, ramp, hot);
      }
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => {
      disposed = true;
      window.removeEventListener("resize", resize);
      window.cancelAnimationFrame(frame);
    };
  }, [paused]);

  return <canvas ref={canvasRef} className="auth-thermal-canvas" aria-hidden="true" />;
}
