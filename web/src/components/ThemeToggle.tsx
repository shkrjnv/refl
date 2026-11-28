import { useTranslation } from "../i18n";
import { isDarkPalette, type PaletteName } from "../theme/tokens";
import { StudioIcon } from "./StudioIcons";

export interface ThemeToggleProps {
  theme: PaletteName;
  onThemeChange: (theme: PaletteName) => void;
  className?: string;
}

/** The prototype has two schemes: the light and dark base palettes. */
export function ThemeToggle({
  theme,
  onThemeChange,
  className = "",
}: ThemeToggleProps) {
  const { t } = useTranslation();
  const dark = isDarkPalette(theme);
  const label = dark ? t("theme-switch-to-light") : t("theme-switch-to-dark");
  return (
    <button
      type="button"
      className={`theme-toggle ${className}`.trim()}
      aria-label={label}
      title={label}
      data-mode={dark ? "dark" : "light"}
      onClick={() => onThemeChange(dark ? "cream-paper" : "deep-umber")}
    >
      <StudioIcon name={dark ? "sun" : "moon"} size={18} />
    </button>
  );
}
