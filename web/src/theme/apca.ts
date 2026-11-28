export interface Rgb {
  r: number;
  g: number;
  b: number;
}

const MAIN_TRC = 2.4;
const R_CO = 0.2126729;
const G_CO = 0.7151522;
const B_CO = 0.072175;
const NORM_BG = 0.56;
const NORM_TXT = 0.57;
const REV_BG = 0.65;
const REV_TXT = 0.62;
const BLACK_THRESHOLD = 0.022;
const BLACK_CLAMP = 1.414;
const SCALE_BOW = 1.14;
const SCALE_WOB = 1.14;
const LOW_OFFSET = 0.027;
const DELTA_MIN = 0.0005;
const LOW_CLIP = 0.1;

export function parseHex(hex: string): Rgb {
  const value = hex.trim().replace(/^#/, "");
  const expanded =
    value.length === 3
      ? value
          .split("")
          .map((part) => `${part}${part}`)
          .join("")
      : value;
  if (!/^[0-9a-fA-F]{6}$/.test(expanded)) {
    throw new Error(`Unsupported colour value: ${hex}`);
  }
  return {
    r: Number.parseInt(expanded.slice(0, 2), 16),
    g: Number.parseInt(expanded.slice(2, 4), 16),
    b: Number.parseInt(expanded.slice(4, 6), 16),
  };
}

export function luminance(rgb: Rgb): number {
  const channel = (value: number) => Math.pow(value / 255, MAIN_TRC);
  return (
    R_CO * channel(rgb.r) + G_CO * channel(rgb.g) + B_CO * channel(rgb.b)
  );
}

function softClamp(value: number): number {
  return value > BLACK_THRESHOLD
    ? value
    : value + Math.pow(BLACK_THRESHOLD - value, BLACK_CLAMP);
}

/**
 * APCA 0.1.9 lightness contrast in Lc units. The sign follows APCA:
 * positive for dark text on a light background, negative for light on dark.
 */
export function apcaLc(text: string, background: string): number {
  const textY = softClamp(luminance(parseHex(text)));
  const backgroundY = softClamp(luminance(parseHex(background)));
  if (Math.abs(backgroundY - textY) < DELTA_MIN) {
    return 0;
  }
  if (backgroundY > textY) {
    const sapc =
      (Math.pow(backgroundY, NORM_BG) - Math.pow(textY, NORM_TXT)) * SCALE_BOW;
    return (sapc < LOW_CLIP ? 0 : sapc - LOW_OFFSET) * 100;
  }
  const sapc =
    (Math.pow(backgroundY, REV_BG) - Math.pow(textY, REV_TXT)) * SCALE_WOB;
  return (sapc > -LOW_CLIP ? 0 : sapc + LOW_OFFSET) * 100;
}

export function apcaAbsolute(text: string, background: string): number {
  return Math.abs(apcaLc(text, background));
}
