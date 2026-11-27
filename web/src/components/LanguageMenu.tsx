import { useState } from "react";

import type { Locale } from "../i18n";
import { useTranslation } from "../i18n";
import { StudioIcon } from "./StudioIcons";

export interface LanguageMenuProps {
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  className?: string;
}

export function LanguageMenu({
  locale,
  locales,
  onLocaleChange,
  className = "",
}: LanguageMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [highlighted, setHighlighted] = useState<Locale | null>(null);
  const highlightedLocale = highlighted ?? locale;

  return (
    <div
      className={`language-menu ${className}`.trim()}
      data-open={open ? "true" : "false"}
      onPointerLeave={() => {
        setOpen(false);
        setHighlighted(null);
      }}
    >
      <button
        type="button"
        className="language-current"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={t("app-language")}
        onClick={() => setOpen((value) => !value)}
      >
        <StudioIcon name="globe" size={16} />
        <span>{locale.toUpperCase()}</span>
        <StudioIcon name="chevron-down" size={12} />
      </button>
      <div
        className="language-options"
        role="listbox"
        aria-label={t("app-language")}
      >
        {locales.map((candidate) => (
          <button
            key={candidate}
            type="button"
            role="option"
            className={`language-option${highlightedLocale === candidate ? " is-highlight" : ""}${candidate === locale ? " active" : ""}`}
            aria-selected={candidate === locale}
            aria-label={t(`language-${candidate}`)}
            title={t(`language-${candidate}`)}
            onPointerEnter={() => setHighlighted(candidate)}
            onPointerLeave={() => setHighlighted(null)}
            onFocus={() => setHighlighted(candidate)}
            onBlur={() => setHighlighted(null)}
            onClick={() => {
              onLocaleChange(candidate);
              setHighlighted(null);
              setOpen(false);
            }}
          >
            {candidate.toUpperCase()}
          </button>
        ))}
      </div>
    </div>
  );
}
