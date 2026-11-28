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
    cx: 0.74,
    cy: 0.18,
    ax: 0.055,
    ay: 0.04,
    sx: 0.043,
    sy: 0.032,
    radius: 0.19,
    phase: 0.6,
    amplitude: 1.75,
    stretchX: 1.4,
    stretchY: 0.78,
    rotation: -0.42,
  },
  {
    cx: 0.17,
    cy: 0.42,
    ax: 0.045,
    ay: 0.05,
    sx: 0.029,
    sy: 0.047,
    radius: 0.155,
    phase: 2.2,
    amplitude: 1.2,
    stretchX: 0.78,
    stretchY: 1.5,
    rotation: 0.38,
  },
  {
    cx: 0.64,
    cy: 0.58,
    ax: 0.075,
    ay: 0.03,
    sx: 0.052,
    sy: 0.031,
    radius: 0.17,
    phase: 3.7,
    amplitude: 1.3,
    stretchX: 1.22,
    stretchY: 0.85,
    rotation: 0.2,
  },
  {
    cx: 0.46,
    cy: 0.79,
    ax: 0.045,
    ay: 0.05,
    sx: 0.035,
    sy: 0.045,
    radius: 0.12,
    phase: 5.1,
    amplitude: 0.92,
    stretchX: 1.1,
    stretchY: 0.95,
    rotation: 0.3,
  },
];

interface PreparedPlume {
  cx: number;
  cy: number;
  cosine: number;
  sine: number;
  stretchX: number;
  stretchY: number;
  spread: number;
  amplitude: number;
}

interface Warmth {
  x: number;
  y: number;
  weight: number;
  targetX: number;
  targetY: number;
  targetWeight: number;
}

function preparePlumes(time: number, warmth: Warmth): PreparedPlume[] {
  return PLUMES.map((plume, index) => {
    const radius =
      plume.radius * (0.92 + Math.sin(time * 0.05 + plume.phase) * 0.08);
    let cx =
      plume.cx +
      Math.sin(time * plume.sx + plume.phase) * plume.ax +
      Math.sin(time * plume.sx * 0.43 + plume.phase * 2.3) * plume.ax * 0.35;
    let cy =
      plume.cy +
      Math.cos(time * plume.sy + plume.phase) * plume.ay +
      Math.cos(time * plume.sy * 0.37 + plume.phase * 1.7) * plume.ay * 0.3;
    let amplitude = plume.amplitude;
    if (index === 3) {
      const pull = warmth.weight * 0.26;
      cx += (warmth.x - cx) * pull;
      cy += (warmth.y - cy) * pull;
      amplitude *= 1 + warmth.weight * 0.22;
    }
    return {
      cx,
      cy,
      cosine: Math.cos(plume.rotation),
      sine: Math.sin(plume.rotation),
      stretchX: plume.stretchX,
      stretchY: plume.stretchY,
      spread: 2 * radius * radius,
      amplitude,
    };
  });
}

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
  if (value <= 0.14) {
    return mix(ramp[0] ?? hot, ramp[1] ?? hot, value / 0.14);
  }
  if (value <= 0.3) {
    return mix(ramp[1] ?? hot, ramp[2] ?? hot, (value - 0.14) / 0.16);
  }
  if (value <= 0.46) {
    return mix(ramp[2] ?? hot, ramp[3] ?? hot, (value - 0.3) / 0.16);
  }
  if (value <= 0.62) {
    return mix(ramp[3] ?? hot, ramp[4] ?? hot, (value - 0.46) / 0.16);
  }
  if (value <= 0.78) {
    return mix(ramp[4] ?? hot, ramp[5] ?? hot, (value - 0.62) / 0.16);
  }
  return mix(ramp[5] ?? hot, hot, (value - 0.78) / 0.22);
}

function fieldAt(
  x: number,
  y: number,
  time: number,
  plumes: readonly PreparedPlume[],
): number {
  // Three low-frequency waves bend the domain so the heat sources blend into
  // irregular plasma instead of concentric ovals. Source drift, elongation, and
  // rotation are prepared once per frame.
  const warpX =
    Math.sin(y * 5.8 + time * 0.083) * 0.018 +
    Math.sin(x * 3.4 - time * 0.052) * 0.012 +
    Math.sin((x + y) * 2.2 + time * 0.031) * 0.01;
  const warpY =
    Math.cos(x * 5.1 - time * 0.071) * 0.017 +
    Math.cos(y * 3.2 + time * 0.058) * 0.011 +
    Math.cos((x - y) * 1.9 - time * 0.037) * 0.009;
  let total = 0.08 + 0.03 * Math.sin(x * 3.1 + y * 2.4 + time * 0.045);
  for (const plume of plumes) {
    const dx = x + warpX - plume.cx;
    const dy = y + warpY - plume.cy;
    const localX = (dx * plume.cosine + dy * plume.sine) / plume.stretchX;
    const localY = (-dx * plume.sine + dy * plume.cosine) / plume.stretchY;
    total +=
      plume.amplitude *
      Math.exp(-(localX * localX + localY * localY) / plume.spread);
  }
  const flow =
    Math.sin(x * 8.7 + y * 12.3 + time * 0.07) *
    Math.cos(y * 6.9 - x * 9.8 - time * 0.055);
  total *= 0.84 + 0.2 * flow;
  return Math.min(1, Math.max(0, total * 0.44));
}

function sensorGrain(x: number, y: number, time: number): number {
  let value =
    (x * 374761393 + y * 668265263 + Math.floor(time * 12) * 1442695041) | 0;
  value = (value ^ (value >>> 13)) * 1274126177 | 0;
  return (((value ^ (value >>> 16)) & 255) / 255 - 0.5) * 10;
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
  warmth: Warmth,
): void {
  const image = context.createImageData(width, height);
  const data = image.data;
  const plumes = preparePlumes(time, warmth);
  for (let y = 0; y < height; y += 1) {
    const ny = y / height;
    for (let x = 0; x < width; x += 1) {
      const nx = x / width;
      const value = fieldAt(nx, ny, time, plumes);
      const color = colorAt(value, ramp, hot);
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

export function AuthBackdrop({
  paused = false,
  focus = null,
}: {
  paused?: boolean;
  focus?: { x: number; y: number } | null;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const lastTimeRef = useRef(0);
  const focusRef = useRef(focus);
  focusRef.current = focus;
  const pointerActiveRef = useRef(false);
  const warmthRef = useRef<Warmth>({
    x: 0.5,
    y: 0.5,
    weight: 0,
    targetX: 0.5,
    targetY: 0.5,
    targetWeight: 0,
  });

  useEffect(() => {
    const reduced =
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) {
      return;
    }
    const warmth = warmthRef.current;
    if (focus) {
      warmth.targetX = focus.x;
      warmth.targetY = focus.y;
      warmth.targetWeight = 1;
    } else if (!pointerActiveRef.current) {
      warmth.targetWeight = 0;
    }
  }, [focus]);

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
    const base = parseHex(cssColor("--thermal-base"), [24, 4, 36]);
    const violet = parseHex(cssColor("--thermal-violet"), [59, 13, 107]);
    const magenta = parseHex(cssColor("--thermal-magenta"), [157, 23, 77]);
    const crimson = parseHex(cssColor("--thermal-crimson"), [217, 30, 54]);
    const orange = parseHex(cssColor("--thermal-orange"), [234, 88, 12]);
    const gold = parseHex(cssColor("--thermal-gold"), [251, 192, 45]);
    const hot = parseHex(cssColor("--thermal-hot"), [255, 242, 199]);
    const ramp: readonly Rgb[] = [base, violet, magenta, crimson, orange, gold];
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
        warmthRef.current,
      );
    };

    resize();
    window.addEventListener("resize", resize);
    if (paused || reduced || typeof window.requestAnimationFrame !== "function") {
      return () => window.removeEventListener("resize", resize);
    }

    const surface = canvas.parentElement ?? canvas;
    const onPointerMove = (event: PointerEvent) => {
      const rect = canvas.getBoundingClientRect();
      if (rect.width < 1 || rect.height < 1) {
        return;
      }
      const warmth = warmthRef.current;
      pointerActiveRef.current = true;
      warmth.targetX = Math.max(
        0,
        Math.min(1, (event.clientX - rect.left) / rect.width),
      );
      warmth.targetY = Math.max(
        0,
        Math.min(1, (event.clientY - rect.top) / rect.height),
      );
      warmth.targetWeight = 1;
    };
    const onPointerLeave = () => {
      pointerActiveRef.current = false;
      const warmth = warmthRef.current;
      const activeFocus = focusRef.current;
      if (activeFocus) {
        warmth.targetX = activeFocus.x;
        warmth.targetY = activeFocus.y;
        warmth.targetWeight = 1;
      } else {
        warmth.targetWeight = 0;
      }
    };
    surface.addEventListener("pointermove", onPointerMove);
    surface.addEventListener("pointerleave", onPointerLeave);

    const tick = (now: number) => {
      if (disposed) {
        return;
      }
      if (now - last >= 45) {
        last = now;
        lastTimeRef.current = now / 1000;
        const warmth = warmthRef.current;
        warmth.x += (warmth.targetX - warmth.x) * 0.08;
        warmth.y += (warmth.targetY - warmth.y) * 0.08;
        warmth.weight += (warmth.targetWeight - warmth.weight) * 0.06;
        drawFrame(
          context,
          canvas.width,
          canvas.height,
          lastTimeRef.current,
          ramp,
          hot,
          warmth,
        );
      }
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => {
      disposed = true;
      window.removeEventListener("resize", resize);
      surface.removeEventListener("pointermove", onPointerMove);
      surface.removeEventListener("pointerleave", onPointerLeave);
      window.cancelAnimationFrame(frame);
    };
  }, [paused]);

  return <canvas ref={canvasRef} className="auth-thermal-canvas" aria-hidden="true" />;
}
