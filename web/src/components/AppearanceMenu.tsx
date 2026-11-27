import { useEffect, useRef, useState } from "react";

import { useTranslation } from "../i18n";
import {
  skinLabelKeys,
  skinNames,
  type SkinName,
  type ThemeName,
} from "../theme/tokens";
import { StudioIcon } from "./StudioIcons";

export interface AppearanceMenuProps {
  theme: ThemeName;
  skin: SkinName;
  onThemeChange: (theme: ThemeName) => void;
  onSkinChange: (skin: SkinName) => void;
  className?: string;
}

export interface PaletteToggleProps {
  theme: ThemeName;
  onThemeChange: (theme: ThemeName) => void;
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
      <button
        type="button"
        className={`palette-choice${theme === "dark" ? " active" : ""}`}
        aria-pressed={theme === "dark"}
        aria-label={t("palette-deep-umber")}
        onClick={() => onThemeChange("dark")}
      >
        <span className="palette-swatch palette-swatch-umber" aria-hidden="true" />
        <span>{t("palette-deep-umber")}</span>
      </button>
      <button
        type="button"
        className={`palette-choice${theme === "light" ? " active" : ""}`}
        aria-pressed={theme === "light"}
        aria-label={t("palette-cream-paper")}
        onClick={() => onThemeChange("light")}
      >
        <span className="palette-swatch palette-swatch-paper" aria-hidden="true" />
        <span>{t("palette-cream-paper")}</span>
      </button>
    </div>
  );
}

export function AppearanceMenu({
  theme,
  skin,
  onThemeChange,
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
        aria-label={t("app-appearance")}
        onClick={() => setOpen((value) => !value)}
      >
        <StudioIcon name="palette" size={18} />
        <span
          className={`appearance-swatch skin-swatch-${skin}`}
          aria-hidden="true"
        />
        <span className="appearance-trigger-label">{t(skinLabelKeys[skin])}</span>
        <StudioIcon name="chevron-down" size={14} />
      </button>
      {open ? (
        <div
          className="appearance-popover"
          role="dialog"
          aria-label={t("app-appearance")}
        >
          <div className="appearance-section">
            <p className="appearance-label">{t("app-skin")}</p>
            <div className="appearance-skins" role="group" aria-label={t("app-skin")}>
              {skinNames.map((candidate) => (
                <button
                  key={candidate}
                  type="button"
                  className={`appearance-skin-option${skin === candidate ? " selected" : ""}`}
                  aria-pressed={skin === candidate}
                  onClick={() => onSkinChange(candidate)}
                >
                  <span
                    className={`appearance-swatch skin-swatch-${candidate}`}
                    aria-hidden="true"
                  />
                  <span>{t(skinLabelKeys[candidate])}</span>
                  {skin === candidate ? (
                    <StudioIcon name="check" size={15} className="appearance-check" />
                  ) : null}
                </button>
              ))}
            </div>
          </div>
          <div className="appearance-section">
            <p className="appearance-label">{t("app-palette")}</p>
            <div className="appearance-palettes">
              <button
                type="button"
                className={`appearance-palette-option${theme === "dark" ? " selected" : ""}`}
                aria-pressed={theme === "dark"}
                onClick={() => onThemeChange("dark")}
              >
                <span
                  className="palette-swatch palette-swatch-umber"
                  aria-hidden="true"
                />
                <span>
                  <strong>{t("palette-deep-umber")}</strong>
                  <small>{t("palette-deep-umber-note")}</small>
                </span>
              </button>
              <button
                type="button"
                className={`appearance-palette-option${theme === "light" ? " selected" : ""}`}
                aria-pressed={theme === "light"}
                onClick={() => onThemeChange("light")}
              >
                <span
                  className="palette-swatch palette-swatch-paper"
                  aria-hidden="true"
                />
                <span>
                  <strong>{t("palette-cream-paper")}</strong>
                  <small>{t("palette-cream-paper-note")}</small>
                </span>
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
