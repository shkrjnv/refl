import { useEffect, useRef } from "react";

import type { SkinName, ThemeName } from "../theme/tokens";

export interface AuthBackdropProps {
  theme: ThemeName;
  skin: SkinName;
}

interface Ribbon {
  color: string;
  accent: string;
  line: number;
  phase: number;
  speed: number;
}

function reducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function cssColor(name: string, fallback = "transparent"): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return value || fallback;
}

function ribbonGradient(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  ribbon: Ribbon,
): CanvasGradient {
  const gradient = context.createLinearGradient(
    -width * 0.1,
    0,
    width * 1.1,
    height,
  );
  gradient.addColorStop(0, "transparent");
  gradient.addColorStop(0.22, ribbon.color);
  gradient.addColorStop(0.58, ribbon.accent);
  gradient.addColorStop(0.84, ribbon.color);
  gradient.addColorStop(1, "transparent");
  return gradient;
}

function drawFrame(
  context: CanvasRenderingContext2D,
  width: number,
  height: number,
  time: number,
  ribbons: Ribbon[],
  top: string,
  bottom: string,
) {
  context.clearRect(0, 0, width, height);
  const base = context.createLinearGradient(0, 0, width, height);
  base.addColorStop(0, top);
  base.addColorStop(0.58, top);
  base.addColorStop(1, bottom);
  context.fillStyle = base;
  context.fillRect(0, 0, width, height);

  context.save();
  context.globalCompositeOperation = "screen";
  for (const ribbon of ribbons) {
    const shift = Math.sin(time * ribbon.speed + ribbon.phase);
    const lift = Math.cos(time * ribbon.speed * 0.72 + ribbon.phase) * height * 0.035;
    const y = height * ribbon.line + shift * height * 0.03 + lift;
    context.strokeStyle = ribbonGradient(context, width, height, ribbon);
    context.lineCap = "round";
    context.lineJoin = "round";
    context.globalAlpha = 0.24;
    context.lineWidth = width * (0.055 + ribbon.line * 0.14);
    context.shadowBlur = Math.max(18, width * 0.045);
    context.shadowColor = ribbon.accent;
    context.beginPath();
    context.moveTo(-width * 0.15, y);
    context.bezierCurveTo(
      width * 0.22,
      y - height * 0.34,
      width * 0.58,
      y + height * 0.26,
      width * 1.15,
      y - height * 0.12,
    );
    context.stroke();

    context.globalAlpha = 0.34;
    context.lineWidth = 1.2;
    context.shadowBlur = 0;
    context.strokeStyle = ribbon.accent;
    context.beginPath();
    context.moveTo(-width * 0.15, y);
    context.bezierCurveTo(
      width * 0.22,
      y - height * 0.34,
      width * 0.58,
      y + height * 0.26,
      width * 1.15,
      y - height * 0.12,
    );
    context.stroke();
  }
  context.restore();

  context.save();
  context.globalAlpha = 0.16;
  context.strokeStyle = cssColor("--auth-panel-text");
  context.lineWidth = 1;
  const ringShift = Math.sin(time * 0.08) * 0.04;
  context.beginPath();
  context.ellipse(
    width * 0.9,
    height * (0.08 + ringShift),
    width * 0.26,
    height * 0.18,
    -0.28,
    0.2,
    Math.PI * 1.35,
  );
  context.stroke();
  context.beginPath();
  context.ellipse(
    width * 0.08,
    height * (0.9 - ringShift),
    width * 0.24,
    height * 0.16,
    0.24,
    Math.PI * 0.9,
    Math.PI * 2.05,
  );
  context.stroke();
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
    const ribbons: Ribbon[] = [
      {
        color: cssColor("--sun"),
        accent: cssColor("--coral"),
        line: 0.2,
        phase: 0.6,
        speed: 0.11,
      },
      {
        color: cssColor("--blue"),
        accent: cssColor("--auth-panel-text"),
        line: 0.44,
        phase: 2.3,
        speed: 0.08,
      },
      {
        color: cssColor("--coral"),
        accent: cssColor("--sun"),
        line: 0.68,
        phase: 4.4,
        speed: 0.1,
      },
      {
        color: cssColor("--auth-panel-text"),
        accent: cssColor("--blue"),
        line: 0.88,
        phase: 5.6,
        speed: 0.07,
      },
    ];
    const top = cssColor("--auth-panel");
    const bottom = cssColor("--bg");

    const resize = () => {
      const ratio = Math.min(1.5, window.devicePixelRatio || 1);
      width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
      height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      drawFrame(
        context,
        width,
        height,
        staticFrame ? 1.7 : last / 1000,
        ribbons,
        top,
        bottom,
      );
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
        drawFrame(context, width, height, now / 1000, ribbons, top, bottom);
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
