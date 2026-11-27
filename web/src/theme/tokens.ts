export const palette = {
  ink: "#14213d",
  bg: "#f3f5fa",
  blue: "#2f6bff",
  sun: "#ff9f43",
  green: "#22b07d",
  coral: "#ff6b6b",
  purp: "#7c5cff",
} as const;

export type Palette = { [Key in keyof typeof palette]: string };
export type ThemeName = "light" | "dark";

export const skinNames = [
  "living-studio",
  "neo-brutalism",
  "soft-neo-brutalism",
  "soft-bento",
  "laboratory",
] as const;

export type SkinName = (typeof skinNames)[number];

export const DEFAULT_SKIN: SkinName = "living-studio";

export const skinLabelKeys: Record<SkinName, string> = {
  "living-studio": "skin-living-studio",
  "neo-brutalism": "skin-neo-brutalism",
  "soft-neo-brutalism": "skin-soft-neo-brutalism",
  "soft-bento": "skin-soft-bento",
  laboratory: "skin-laboratory",
};

export const skinDescriptionKeys: Record<SkinName, string> = {
  "living-studio": "skin-living-studio-note",
  "neo-brutalism": "skin-neo-brutalism-note",
  "soft-neo-brutalism": "skin-soft-neo-brutalism-note",
  "soft-bento": "skin-soft-bento-note",
  laboratory: "skin-laboratory-note",
};

export interface ThemeTokens extends Palette {
  surface: string;
  surfaceRaised: string;
  surfaceMuted: string;
  border: string;
  borderStrong: string;
  text: string;
  textMuted: string;
  blueDeep: string;
  sunDeep: string;
  greenDeep: string;
  coralDeep: string;
  purpDeep: string;
  focusRing: string;
  shadowButton: string;
  shadowPrimary: string;
  shadowCard: string;
  shadowHero: string;
  shadowSoft: string;
  overlay: string;
  infoSurface: string;
  infoBorder: string;
  warningSurface: string;
  warningBorder: string;
  successSurface: string;
  successBorder: string;
  dangerSurface: string;
  dangerBorder: string;
  iellaInk: string;
  iellaBeak: string;
  iellaCream: string;
  iellaBrown: string;
  iellaEye: string;
}

export const themes: Record<ThemeName, ThemeTokens> = {
  light: {
    ...palette,
    surface: "#ffffff",
    surfaceRaised: "#ffffff",
    surfaceMuted: "#eef1fb",
    border: "#e3e8f2",
    borderStrong: "#c7cfe6",
    text: "#14213d",
    textMuted: "#6b7489",
    blueDeep: "#1d4fd1",
    sunDeep: "#e07d1f",
    greenDeep: "#178a60",
    coralDeep: "#d94f4f",
    purpDeep: "#5b3fe0",
    focusRing: "#2f6bff",
    shadowButton: "0 4px 0 #c7cfe6",
    shadowPrimary: "0 4px 0 #1d4fd1",
    shadowCard: "0 6px 0 #e3e8f2",
    shadowHero: "0 8px 0 #c7cfe6",
    shadowSoft: "0 18px 45px rgba(20, 33, 61, 0.12)",
    overlay: "rgba(20, 33, 61, 0.45)",
    infoSurface: "#eef3ff",
    infoBorder: "#d6e1ff",
    warningSurface: "#fff4e6",
    warningBorder: "#ffe0bd",
    successSurface: "#eaf8f1",
    successBorder: "#ccefe1",
    dangerSurface: "#fff0f0",
    dangerBorder: "#ffd2d2",
    iellaInk: "#24346a",
    iellaBeak: "#e0773c",
    iellaCream: "#f3e3c3",
    iellaBrown: "#8b5a2b",
    iellaEye: "#0e1020",
  },
  dark: {
    ...palette,
    bg: "#0e1626",
    surface: "#151f36",
    surfaceRaised: "#1b2742",
    surfaceMuted: "#202d4b",
    border: "#2a3854",
    borderStrong: "#3a4b70",
    text: "#eef3ff",
    textMuted: "#9aa8c7",
    blue: "#5b8cff",
    sun: "#ffb35c",
    green: "#3bd39a",
    coral: "#ff8585",
    purp: "#9a80ff",
    blueDeep: "#2f6bff",
    sunDeep: "#e08a2f",
    greenDeep: "#1f9f6f",
    coralDeep: "#da5c5c",
    purpDeep: "#6f52ef",
    focusRing: "#8fb0ff",
    shadowButton: "0 4px 0 #0a0f1c",
    shadowPrimary: "0 4px 0 #1d3f99",
    shadowCard: "0 6px 0 #0a0f1c",
    shadowHero: "0 8px 0 #0a0f1c",
    shadowSoft: "0 18px 45px rgba(0, 0, 0, 0.35)",
    overlay: "rgba(0, 0, 0, 0.6)",
    infoSurface: "#172646",
    infoBorder: "#2d4470",
    warningSurface: "#342511",
    warningBorder: "#62451c",
    successSurface: "#122f25",
    successBorder: "#235743",
    dangerSurface: "#351a20",
    dangerBorder: "#66303b",
    iellaInk: "#344b8f",
    iellaBeak: "#f08a4b",
    iellaCream: "#f3e3c3",
    iellaBrown: "#a46a35",
    iellaEye: "#0e1020",
  },
};

const THEME_KEY = "refl.theme";
const SKIN_KEY = "refl.skin";

export function isThemeName(value: string | null): value is ThemeName {
  return value === "light" || value === "dark";
}

export function isSkinName(value: string | null): value is SkinName {
  return value !== null && skinNames.includes(value as SkinName);
}

export function systemTheme(): ThemeName {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return "light";
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function readStoredTheme(): ThemeName | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const value = window.localStorage.getItem(THEME_KEY);
    return isThemeName(value) ? value : null;
  } catch {
    return null;
  }
}

export function storeTheme(theme: ThemeName): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

export function initialTheme(): ThemeName {
  return readStoredTheme() ?? systemTheme();
}

export function readStoredSkin(): SkinName | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const value = window.localStorage.getItem(SKIN_KEY);
    return isSkinName(value) ? value : null;
  } catch {
    return null;
  }
}

export function storeSkin(skin: SkinName): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(SKIN_KEY, skin);
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

export function initialSkin(): SkinName {
  return readStoredSkin() ?? DEFAULT_SKIN;
}

export function applyTheme(
  theme: ThemeName,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.theme = theme;
  root.style.colorScheme = theme;
}

export function applySkin(
  skin: SkinName,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset.skin = skin;
}

export function applyAppearance(
  theme: ThemeName,
  skin: SkinName,
  root: HTMLElement = document.documentElement,
): void {
  applyTheme(theme, root);
  applySkin(skin, root);
}
