import { useEffect, useRef, useState } from "react";

import { useTranslation } from "../i18n";
import {
  paletteLabelKeys,
  paletteNames,
  skinLabelKeys,
  skinNames,
  type PaletteName,
  type SkinName,
  type ThemeName,
} from "../theme/tokens";
import { StudioIcon } from "./StudioIcons";

export interface AppearanceMenuProps {
  skin: SkinName;
  onSkinChange: (skin: SkinName) => void;
  className?: string;
}

export interface PaletteToggleProps {
  theme: ThemeName;
  onThemeChange: (theme: PaletteName) => void;
  className?: string;
}

export function PaletteToggle({
  theme,
  onThemeChange,
  className = "",
}: PaletteToggleProps) {
  const { t } = useTranslation();
  return (
    <div
      className={`palette-toggle ${className}`.trim()}
      role="group"
      aria-label={t("app-palette")}
    >
      {paletteNames.map((candidate) => (
        <button
          key={candidate}
          type="button"
          className={`palette-choice${theme === candidate ? " active" : ""}`}
          aria-pressed={theme === candidate}
          aria-label={t(paletteLabelKeys[candidate])}
          title={t(paletteLabelKeys[candidate])}
          onClick={() => onThemeChange(candidate)}
        >
          <span
            className={`appearance-chip-swatch appearance-chip-swatch-tiny palette-swatch-${candidate}`}
            aria-hidden="true"
          >
            <span />
          </span>
        </button>
      ))}
    </div>
  );
}

export function AppearanceMenu({
  skin,
  onSkinChange,
  className = "",
}: AppearanceMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    const onPointerDown = (event: PointerEvent) => {
      if (
        event.target instanceof Node &&
        !rootRef.current?.contains(event.target)
      ) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  return (
    <div className={`appearance-menu ${className}`.trim()} ref={rootRef}>
      <button
        type="button"
        className="appearance-trigger"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={t("app-skin")}
        onClick={() => setOpen((value) => !value)}
      >
        <StudioIcon name="palette" size={17} />
        <span className="appearance-trigger-label">
          {t(skinLabelKeys[skin])}
        </span>
        <StudioIcon name="chevron-down" size={13} />
      </button>
      {open ? (
        <div
          className="appearance-popover appearance-popover-compact"
          role="dialog"
          aria-label={t("app-skin")}
        >
          <section className="appearance-compact-section">
            <p className="appearance-label">{t("app-skin")}</p>
            <div
              className="appearance-skins appearance-skins-compact"
              role="group"
              aria-label={t("app-skin")}
            >
              {skinNames.map((candidate) => (
                <button
                  key={candidate}
                  type="button"
                  className={`appearance-skin-option appearance-skin-option-compact${skin === candidate ? " selected" : ""}`}
                  aria-pressed={skin === candidate}
                  onClick={() => onSkinChange(candidate)}
                >
                  <span
                    className={`appearance-swatch appearance-swatch-small skin-swatch-${candidate}`}
                    aria-hidden="true"
                  />
                  <span>{t(skinLabelKeys[candidate])}</span>
                  {skin === candidate ? (
                    <StudioIcon name="check" size={15} className="appearance-check" />
                  ) : null}
                </button>
              ))}
            </div>
          </section>
        </div>
      ) : null}
    </div>
  );
}
