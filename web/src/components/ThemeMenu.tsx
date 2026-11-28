import { useEffect, useRef, useState } from "react";

import { useTranslation } from "../i18n";
import { paletteLabelKeys, paletteNames, type PaletteName } from "../theme/tokens";
import { StudioIcon } from "./StudioIcons";

export interface ThemeMenuProps {
  theme: PaletteName;
  onThemeChange: (theme: PaletteName) => void;
  className?: string;
}

/** Compact header control: swatches only, no internal palette slugs. */
export function ThemeMenu({
  theme,
  onThemeChange,
  className = "",
}: ThemeMenuProps) {
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
    <div
      className={`theme-menu ${className}`.trim()}
      data-open={open ? "true" : "false"}
      ref={rootRef}
      onBlur={(event) => {
        if (
          event.relatedTarget instanceof Node &&
          !event.currentTarget.contains(event.relatedTarget)
        ) {
          setOpen(false);
        }
      }}
    >
      <button
        type="button"
        className="theme-menu-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`${t("app-theme")}: ${t(paletteLabelKeys[theme])}`}
        title={`${t("app-theme")}: ${t(paletteLabelKeys[theme])}`}
        onClick={() => setOpen((value) => !value)}
      >
        <span
          className={`palette-swatch palette-swatch-${theme}`}
          aria-hidden="true"
        />
        <StudioIcon name="chevron-down" size={12} />
      </button>
      <div
        className="theme-menu-options"
        role="listbox"
        aria-label={t("app-theme")}
      >
        {paletteNames.map((candidate) => (
          <button
            key={candidate}
            type="button"
            role="option"
            className={`theme-menu-option${candidate === theme ? " active" : ""}`}
            aria-selected={candidate === theme}
            aria-label={t(paletteLabelKeys[candidate])}
            title={t(paletteLabelKeys[candidate])}
            onClick={() => {
              onThemeChange(candidate);
              setOpen(false);
            }}
          >
            <span
              className={`palette-swatch palette-swatch-${candidate}`}
              aria-hidden="true"
            />
            {candidate === theme ? (
              <StudioIcon name="check" size={12} className="theme-menu-check" />
            ) : null}
          </button>
        ))}
      </div>
    </div>
  );
}
