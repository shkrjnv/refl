import { useEffect, useState } from "react";

import { useTranslation } from "../i18n";
import {
  markSectionTipSeen,
  shouldShowSectionTip,
  type SectionTipId,
} from "../prototype/sectionTips";
import { IellaSprite } from "./IellaSprite";
import { StudioIcon } from "./StudioIcons";

export interface IellaTipProps {
  section: SectionTipId;
  className?: string;
}

/**
 * Contextual one-time tip for a section. Mount it with the section screen:
 * the tip marks itself seen on the first mount and never blocks the screen.
 */
export function IellaTip({ section, className = "" }: IellaTipProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(() => shouldShowSectionTip(section));

  useEffect(() => {
    if (open) {
      markSectionTipSeen(section);
    }
  }, [open, section]);

  if (!open) {
    return null;
  }

  return (
    <aside
      className={`iella-tip ${className}`.trim()}
      role="note"
      aria-live="polite"
      aria-label={t("iella-tip-label")}
    >
      <div className="iella-tip-sprite">
        <IellaSprite size={36} />
      </div>
      <p>{t(`tip-${section}`)}</p>
      <button
        type="button"
        className="iella-tip-close"
        aria-label={t("iella-tip-dismiss")}
        onClick={() => setOpen(false)}
      >
        <StudioIcon name="close" size={15} />
      </button>
    </aside>
  );
}
