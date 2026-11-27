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

export const paletteNames = [
  "deep-umber",
  "burgundy",
  "midnight-cobalt",
  "cream-paper",
  "sand-forest",
] as const;

export type PaletteName = (typeof paletteNames)[number];
export type ThemeName = PaletteName;

export const skinNames = [
  "living-studio",
  "neo-brutalism",
  "soft-neo-brutalism",
  "soft-bento",
  "laboratory",
] as const;

export type SkinName = (typeof skinNames)[number];

export const DEFAULT_SKIN: SkinName = "living-studio";
export const DEFAULT_THEME: PaletteName = "deep-umber";

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

export const paletteLabelKeys: Record<PaletteName, string> = {
  "deep-umber": "palette-deep-umber",
  burgundy: "palette-burgundy",
  "cream-paper": "palette-cream-paper",
  "sand-forest": "palette-sand-forest",
  "midnight-cobalt": "palette-midnight-cobalt",
};

export const paletteNoteKeys: Record<PaletteName, string> = {
  "deep-umber": "palette-deep-umber-note",
  burgundy: "palette-burgundy-note",
  "cream-paper": "palette-cream-paper-note",
  "sand-forest": "palette-sand-forest-note",
  "midnight-cobalt": "palette-midnight-cobalt-note",
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

export const themes: Record<PaletteName, ThemeTokens> = {
  "cream-paper": {
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
    iellaInk: "#24346A",
    iellaBeak: "#E0773C",
    iellaCream: "#F3E3C3",
    iellaBrown: "#8B5A2B",
    iellaEye: "#0E1020",
  },
  "deep-umber": {
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
    iellaInk: "#24346A",
    iellaBeak: "#E0773C",
    iellaCream: "#F3E3C3",
    iellaBrown: "#8B5A2B",
    iellaEye: "#0E1020",
  },
  burgundy: {
    ...palette,
    bg: "#1B0B10",
    surface: "#271118",
    surfaceRaised: "#331720",
    surfaceMuted: "#3A1A24",
    surfaceCard: "#271118",
    border: "#4A2532",
    borderStrong: "#6B3849",
    text: "#F5ECE3",
    textMuted: "#C4ABB2",
    blue: "#E03A5D",
    sun: "#E2A652",
    green: "#63C7A2",
    coral: "#FF7C96",
    purp: "#B98CFF",
    blueDeep: "#B22845",
    sunDeep: "#A9782E",
    greenDeep: "#3D9474",
    coralDeep: "#B82E4B",
    purpDeep: "#7A57C7",
    focusRing: "#FF8DA3",
    microAccent: "#E2A652",
    shadowButton: "0 4px 0 #0C0407",
    shadowPrimary: "0 4px 0 #B22845",
    shadowCard: "0 6px 0 #0C0407",
    shadowHero: "0 8px 0 #0C0407",
    shadowSoft: "0 18px 45px rgba(0, 0, 0, 0.46)",
    overlay: "rgba(10, 2, 6, 0.68)",
    infoSurface: "#331822",
    infoBorder: "#5E2E3E",
    warningSurface: "#332417",
    warningBorder: "#635026",
    successSurface: "#173129",
    successBorder: "#2E5C4C",
    dangerSurface: "#35141D",
    dangerBorder: "#6A2838",
    iellaInk: "#24346A",
    iellaBeak: "#E0773C",
    iellaCream: "#F3E3C3",
    iellaBrown: "#8B5A2B",
    iellaEye: "#0E1020",
  },
  "sand-forest": {
    ...palette,
    bg: "#F3E6CD",
    surface: "#FFF9ED",
    surfaceRaised: "#FFFDF8",
    surfaceMuted: "#EADBBD",
    surfaceCard: "#FFF9ED",
    border: "#C9B58C",
    borderStrong: "#183A2E",
    text: "#183A2E",
    textMuted: "#576958",
    blue: "#1F5C48",
    blueDeep: "#113C2E",
    sun: "#E7A438",
    sunDeep: "#8F5A0A",
    green: "#2F7D5A",
    greenDeep: "#1B5B3F",
    coral: "#C5543F",
    coralDeep: "#8A3025",
    purp: "#6F528C",
    purpDeep: "#4E3666",
    focusRing: "#1F5C48",
    microAccent: "#E7A438",
    shadowButton: "0 4px 0 #183A2E",
    shadowPrimary: "0 4px 0 #113C2E",
    shadowCard: "0 6px 0 #183A2E",
    shadowHero: "0 8px 0 #183A2E",
    shadowSoft: "8px 8px 0 rgba(24, 58, 46, 0.18)",
    overlay: "rgba(18, 42, 32, 0.52)",
    infoSurface: "#E4F0E7",
    infoBorder: "#ABC9B8",
    warningSurface: "#FBE8C9",
    warningBorder: "#D8AA62",
    successSurface: "#DFEEE3",
    successBorder: "#A7C8B3",
    dangerSurface: "#F8DDD7",
    dangerBorder: "#D39A8D",
    iellaInk: "#24346A",
    iellaBeak: "#E0773C",
    iellaCream: "#F3E3C3",
    iellaBrown: "#8B5A2B",
    iellaEye: "#0E1020",
  },
  "midnight-cobalt": {
    ...palette,
    bg: "#0D1524",
    surface: "#141F33",
    surfaceRaised: "#1A2942",
    surfaceMuted: "#21324E",
    surfaceCard: "#141F33",
    border: "#2A3E60",
    borderStrong: "#405B87",
    text: "#E9EEF9",
    textMuted: "#A8B6CF",
    blue: "#3D6BFF",
    sun: "#E9B45E",
    green: "#50C9A4",
    coral: "#FF7F94",
    purp: "#9B87FF",
    blueDeep: "#2A4FD1",
    sunDeep: "#B07A28",
    greenDeep: "#2D9C7C",
    coralDeep: "#C04D62",
    purpDeep: "#6D59C9",
    focusRing: "#6E92FF",
    microAccent: "#E9B45E",
    shadowButton: "0 4px 0 #070C15",
    shadowPrimary: "0 4px 0 #2A4FD1",
    shadowCard: "0 6px 0 #070C15",
    shadowHero: "0 8px 0 #070C15",
    shadowSoft: "0 20px 48px rgba(0, 0, 0, 0.44)",
    overlay: "rgba(0, 4, 12, 0.68)",
    infoSurface: "#162B47",
    infoBorder: "#2C4F7A",
    warningSurface: "#332714",
    warningBorder: "#635024",
    successSurface: "#13352C",
    successBorder: "#2A6454",
    dangerSurface: "#371E29",
    dangerBorder: "#6C3546",
    iellaInk: "#24346A",
    iellaBeak: "#E0773C",
    iellaCream: "#F3E3C3",
    iellaBrown: "#8B5A2B",
    iellaEye: "#0E1020",
  },
};

const THEME_KEY = "refl.theme";
const PALETTE_KEY = "refl.palette";
const SKIN_KEY = "refl.skin";

const legacyThemes: Record<string, PaletteName> = {
  light: "cream-paper",
  dark: "deep-umber",
};

export function isPaletteName(value: string | null): value is PaletteName {
  return value !== null && paletteNames.includes(value as PaletteName);
}

export function isThemeName(value: string | null): value is ThemeName {
  return isPaletteName(value) || (value !== null && value in legacyThemes);
}

export function normalizePalette(value: string | null): PaletteName | null {
  if (isPaletteName(value)) {
    return value;
  }
  return value === null ? null : (legacyThemes[value] ?? null);
}

export function isDarkPalette(paletteName: PaletteName): boolean {
  return (
    paletteName === "deep-umber" ||
    paletteName === "burgundy" ||
    paletteName === "midnight-cobalt"
  );
}

export function isSkinName(value: string | null): value is SkinName {
  return value !== null && skinNames.includes(value as SkinName);
}

export function readStoredTheme(): PaletteName | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const current = normalizePalette(window.localStorage.getItem(PALETTE_KEY));
    if (current !== null) {
      return current;
    }
    return normalizePalette(window.localStorage.getItem(THEME_KEY));
  } catch {
    return null;
  }
}

export function storeTheme(theme: PaletteName): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(PALETTE_KEY, theme);
    window.localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Storage can be unavailable in private or blocked contexts.
  }
}

export function initialTheme(): PaletteName {
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
  theme: PaletteName,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset["palette"] = theme;
  root.dataset["theme"] = isDarkPalette(theme) ? "dark" : "light";
  root.style.colorScheme = isDarkPalette(theme) ? "dark" : "light";
}

export function applySkin(
  skin: SkinName,
  root: HTMLElement = document.documentElement,
): void {
  root.dataset["skin"] = skin;
}

export function applyAppearance(
  theme: PaletteName,
  skin: SkinName,
  root: HTMLElement = document.documentElement,
): void {
  applyTheme(theme, root);
  applySkin(skin, root);
}
