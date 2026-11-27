import { useEffect, useRef } from "react";

type Rgb = readonly [number, number, number];

interface Plume {
  cx: number;
  cy: number;
  ax: number;
  ay: number;
  sx: number;
  sy: number;
  radius: number;
  phase: number;
  amplitude: number;
  stretchX: number;
  stretchY: number;
  rotation: number;
}

const PLUMES: readonly Plume[] = [
  {
    cx: 0.72,
    cy: 0.2,
    ax: 0.065,
    ay: 0.045,
    sx: 0.05,
    sy: 0.043,
    radius: 0.19,
    phase: 0.7,
    amplitude: 1.55,
    stretchX: 1.32,
    stretchY: 0.78,
    rotation: -0.38,
  },
  {
    cx: 0.22,
    cy: 0.42,
    ax: 0.075,
    ay: 0.045,
    sx: 0.04,
    sy: 0.061,
    radius: 0.17,
    phase: 2.4,
    amplitude: 1.16,
    stretchX: 0.8,
    stretchY: 1.5,
    rotation: 0.4,
  },
  {
    cx: 0.8,
    cy: 0.62,
    ax: 0.1,
    ay: 0.045,
    sx: 0.06,
    sy: 0.037,
    radius: 0.145,
    phase: 4.1,
    amplitude: 1.02,
    stretchX: 1.28,
    stretchY: 0.82,
    rotation: -0.18,
  },
  {
    cx: 0.27,
    cy: 0.82,
    ax: 0.055,
    ay: 0.07,
    sx: 0.035,
    sy: 0.052,
    radius: 0.15,
    phase: 5.4,
    amplitude: 0.9,
    stretchX: 0.96,
    stretchY: 1.25,
    rotation: 0.3,
  },
];

const CONTOURS = [0.18, 0.32, 0.46, 0.6, 0.74] as const;

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
  if (value <= 0.16) {
    return mix(ramp[0] ?? hot, ramp[1] ?? hot, value / 0.16);
  }
  if (value <= 0.34) {
    return mix(ramp[1] ?? hot, ramp[2] ?? hot, (value - 0.16) / 0.18);
  }
  if (value <= 0.52) {
    return mix(ramp[2] ?? hot, ramp[3] ?? hot, (value - 0.34) / 0.18);
  }
  if (value <= 0.7) {
    return mix(ramp[3] ?? hot, ramp[4] ?? hot, (value - 0.52) / 0.18);
  }
  return mix(ramp[4] ?? hot, hot, (value - 0.7) / 0.3);
}

function fieldAt(x: number, y: number, time: number): number {
  // Two low-frequency waves bend the domain, so isolines never become perfect
  // concentric ovals. Each source also has its own elongation and orientation.
  const warpX =
    Math.sin(y * 7.4 + time * 0.11) * 0.009 +
    Math.sin(x * 5.2 - time * 0.073) * 0.006;
  const warpY =
    Math.cos(x * 6.1 - time * 0.09) * 0.009 +
    Math.cos(y * 4.4 + time * 0.082) * 0.006;
  let total = 0;
  for (const plume of PLUMES) {
    const cx =
      plume.cx +
      Math.sin(time * plume.sx + plume.phase) * plume.ax +
      Math.sin(time * plume.sx * 0.43 + plume.phase * 2.3) * plume.ax * 0.35;
    const cy =
      plume.cy +
      Math.cos(time * plume.sy + plume.phase) * plume.ay +
      Math.cos(time * plume.sy * 0.37 + plume.phase * 1.7) * plume.ay * 0.3;
    const radius =
      plume.radius * (0.92 + Math.sin(time * 0.05 + plume.phase) * 0.08);
    const dx = x + warpX - cx;
    const dy = y + warpY - cy;
    const cosine = Math.cos(plume.rotation);
    const sine = Math.sin(plume.rotation);
    const localX = (dx * cosine + dy * sine) / plume.stretchX;
    const localY = (-dx * sine + dy * cosine) / plume.stretchY;
    total +=
      plume.amplitude *
      Math.exp(-(localX * localX + localY * localY) / (2 * radius * radius));
  }
  return Math.min(1, total * 0.56);
}

function sensorGrain(x: number, y: number, time: number): number {
  let value =
    (x * 374761393 + y * 668265263 + Math.floor(time * 12) * 1442695041) | 0;
  value = (value ^ (value >>> 13)) * 1274126177 | 0;
  return (((value ^ (value >>> 16)) & 255) / 255 - 0.5) * 7;
}

function clampChannel(value: number): number {
  return Math.max(0, Math.min(255, Math.round(value)));
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
        const wobble = Math.sin(nx * 13 + ny * 8 + time * 0.35) * 0.006;
        const distance = Math.abs(value - (contour + wobble));
        if (distance < 0.015) {
          color = mix(color, hot, 0.38 * (1 - distance / 0.015));
        }
      }
      const grain = sensorGrain(x, y, time);
      const offset = (y * width + x) * 4;
      data[offset] = clampChannel(color[0] + grain);
      data[offset + 1] = clampChannel(color[1] + grain);
      data[offset + 2] = clampChannel(color[2] + grain);
      data[offset + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
}

export function AuthBackdrop({ paused = false }: { paused?: boolean }) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const lastTimeRef = useRef(0);

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
    const base = parseHex(cssColor("--thermal-base"), [18, 7, 41]);
    const violet = parseHex(cssColor("--thermal-violet"), [59, 13, 107]);
    const crimson = parseHex(cssColor("--thermal-crimson"), [157, 23, 77]);
    const orange = parseHex(cssColor("--thermal-orange"), [234, 88, 12]);
    const gold = parseHex(cssColor("--thermal-gold"), [250, 204, 21]);
    const hot = parseHex(cssColor("--thermal-hot"), [254, 249, 195]);
    const ramp: readonly Rgb[] = [base, violet, crimson, orange, gold];
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
      drawFrame(
        context,
        width,
        height,
        reduced ? 4.2 : lastTimeRef.current,
        ramp,
        hot,
      );
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
        lastTimeRef.current = now / 1000;
        drawFrame(
          context,
          canvas.width,
          canvas.height,
          lastTimeRef.current,
          ramp,
          hot,
        );
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
