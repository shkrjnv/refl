import { useEffect, useRef } from "react";

import type { SkinName, ThemeName } from "../theme/tokens";

export interface AuthBackdropProps {
  theme: ThemeName;
  skin: SkinName;
}

interface MeshColor {
  color: string;
  weight: number;
  phase: number;
  drift: number;
}

function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function cssColor(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return value || fallback;
}

function drawFrame(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  time: number,
  palette: MeshColor[],
  top: string,
  bottom: string,
) {
  context.clearRect(0, 0, width, height);
  const base = context.createLinearGradient(0, 0, width, height);
  base.addColorStop(0, top);
  base.addColorStop(1, bottom);
  context.fillStyle = base;
  context.fillRect(0, 0, width, height);

  context.save();
  context.globalCompositeOperation = "screen";
  for (const [index, color] of palette.entries()) {
    const x =
      width *
      (0.18 +
        0.64 *
          (0.5 +
            0.5 * Math.sin(time * color.drift + color.phase + index * 0.8)));
    const y =
      height *
      (0.2 +
        0.68 *
          (0.5 +
            0.5 * Math.cos(time * color.drift * 0.82 + color.phase * 1.3)));
    const radius = Math.min(width, height) * (0.28 + color.weight * 0.28);
    const gradient = context.createRadialGradient(x, y, 0, x, y, radius);
    gradient.addColorStop(0, color.color);
    gradient.addColorStop(0.55, color.color);
    gradient.addColorStop(1, "transparent");
    context.globalAlpha = 0.56;
    context.fillStyle = gradient;
    context.beginPath();
    context.arc(x, y, radius, 0, Math.PI * 2);
    context.fill();
    context.globalAlpha = 1;
  }
  context.restore();

  context.save();
  context.globalAlpha = 0.15;
  context.strokeStyle = cssColor("--auth-panel-text", "transparent");
  context.lineWidth = 1;
  for (let line = 1; line < 7; line += 1) {
    context.beginPath();
    for (let x = 0; x <= width; x += 12) {
      const y =
        (height * line) / 7 +
        Math.sin(x / 90 + time * 0.16 + line) * 8 +
        Math.cos(x / 170 + time * 0.11) * 5;
      if (x === 0) {
        context.moveTo(x, y);
      } else {
        context.lineTo(x, y);
      }
    }
    context.stroke();
  }
  context.restore();
}

export function AuthBackdrop({ theme, skin }: AuthBackdropProps) {
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
    let width = 0;
    let height = 0;
    let frame = 0;
    let disposed = false;
    let last = -1;
    const staticFrame = reducedMotion();
    const palette: MeshColor[] = [
      {
        color: cssColor("--blue", "transparent"),
        weight: 0.55,
        phase: 0.4,
        drift: 0.11,
      },
      {
        color: cssColor("--sun", "transparent"),
        weight: 0.38,
        phase: 2.2,
        drift: 0.08,
      },
      {
        color: cssColor("--coral", "transparent"),
        weight: 0.28,
        phase: 4.3,
        drift: 0.09,
      },
      {
        color: cssColor("--purp", "transparent"),
        weight: 0.4,
        phase: 5.4,
        drift: 0.07,
      },
    ];
    const top = cssColor("--auth-panel", "transparent");
    const bottom = cssColor("--bg", "transparent");

    const resize = () => {
      const ratio = Math.min(1.5, window.devicePixelRatio || 1);
      width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
      height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      drawFrame(context, width, height, staticFrame ? 1.7 : last / 1000, palette, top, bottom);
    };

    resize();
    window.addEventListener("resize", resize);
    if (staticFrame || typeof window.requestAnimationFrame !== "function") {
      return () => {
        window.removeEventListener("resize", resize);
      };
    }

    const tick = (now: number) => {
      if (disposed) {
        return;
      }
      if (now - last >= 33) {
        last = now;
        drawFrame(context, width, height, now / 1000, palette, top, bottom);
      }
      frame = window.requestAnimationFrame(tick);
    };
    frame = window.requestAnimationFrame(tick);
    return () => {
      disposed = true;
      window.removeEventListener("resize", resize);
      window.cancelAnimationFrame(frame);
    };
  }, [theme, skin]);

  return <canvas ref={canvasRef} className="auth-backdrop-canvas" aria-hidden="true" />;
}
