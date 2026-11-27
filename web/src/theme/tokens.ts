export const palette = {
  ink: "#1D2424",
  bg: "#FAF4E8",
  blue: "#0D7A78",
  sun: "#D99B52",
  green: "#2F8B72",
  coral: "#FF2E7E",
  purp: "#6C63B5",
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
export const DEFAULT_THEME: ThemeName = "dark";

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
  surfaceCard: string;
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
  microAccent: string;
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
    surface: "#FFFDF8",
    surfaceRaised: "#FFFDF8",
    surfaceMuted: "#F2E8D5",
    surfaceCard: "#FFFDF8",
    border: "#DCCFB8",
    borderStrong: "#C4B298",
    text: "#1D2424",
    textMuted: "#5F665F",
    blueDeep: "#085E5C",
    sunDeep: "#A96F2F",
    greenDeep: "#1F6B57",
    coralDeep: "#C00058",
    purpDeep: "#4E478F",
    focusRing: "#0D7A78",
    microAccent: "#FF2E7E",
    shadowButton: "0 4px 0 #DCCFB8",
    shadowPrimary: "0 4px 0 #085E5C",
    shadowCard: "0 6px 0 #E6DAC4",
    shadowHero: "0 8px 0 #C4B298",
    shadowSoft: "0 18px 45px rgba(29, 36, 36, 0.12)",
    overlay: "rgba(29, 36, 36, 0.42)",
    infoSurface: "#E5F0EE",
    infoBorder: "#B8DAD5",
    warningSurface: "#F7EAD7",
    warningBorder: "#E3C79E",
    successSurface: "#E3F1EA",
    successBorder: "#B7D8C7",
    dangerSurface: "#FFE6EF",
    dangerBorder: "#F4B8CE",
    iellaInk: "#2B4A45",
    iellaBeak: "#C97A3A",
    iellaCream: "#F5E6C8",
    iellaBrown: "#8A5A32",
    iellaEye: "#17201E",
  },
  dark: {
    ...palette,
    bg: "#161210",
    surface: "#201A17",
    surfaceRaised: "#28211D",
    surfaceMuted: "#2E2621",
    surfaceCard: "#201A17",
    border: "#45382E",
    borderStrong: "#6A5644",
    text: "#EFE6D5",
    textMuted: "#B7A892",
    blue: "#2B9AF3",
    sun: "#D99B52",
    green: "#4CC9A0",
    coral: "#FF7A9E",
    purp: "#8E7DFF",
    blueDeep: "#1E79C4",
    sunDeep: "#A96F2F",
    greenDeep: "#2E9C7A",
    coralDeep: "#C34661",
    purpDeep: "#6252B5",
    focusRing: "#4DB2FF",
    microAccent: "#D99B52",
    shadowButton: "0 4px 0 #0B0908",
    shadowPrimary: "0 4px 0 #1E79C4",
    shadowCard: "0 6px 0 #0B0908",
    shadowHero: "0 8px 0 #0B0908",
    shadowSoft: "0 18px 45px rgba(0, 0, 0, 0.4)",
    overlay: "rgba(0, 0, 0, 0.66)",
    infoSurface: "#1B2C3B",
    infoBorder: "#2E526D",
    warningSurface: "#332717",
    warningBorder: "#5D4622",
    successSurface: "#17332B",
    successBorder: "#2D5D4D",
    dangerSurface: "#351F25",
    dangerBorder: "#66343F",
    iellaInk: "#C9A87B",
    iellaBeak: "#E0A35C",
    iellaCream: "#EFE6D5",
    iellaBrown: "#A97A4B",
    iellaEye: "#161210",
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
  return readStoredTheme() ?? DEFAULT_THEME;
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
