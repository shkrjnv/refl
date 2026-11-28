import { useEffect } from "react";

import type { CompanionId } from "../companions/registry";
import { useTranslation } from "../i18n";
import { BrandLogo } from "./StudioIcons";
import { CompanionSprite } from "./CompanionSprite";

export interface BootScreenProps {
  companion: CompanionId;
}

/** Borderless first paint: brand, companion, and one calm progress line. */
export function BootScreen({ companion }: BootScreenProps) {
  const { t } = useTranslation();

  useEffect(() => {
    const previous = document.title;
    document.title = t("app-content-loading");
    return () => {
      document.title = previous;
    };
  }, [t]);

  return (
    <main className="boot-screen" role="status" aria-live="polite">
      <div className="boot-orbit" aria-hidden="true">
        <span className="boot-glow" />
        <span className="boot-brand">
          <BrandLogo size={64} />
        </span>
        <span className="boot-companion">
          <CompanionSprite companion={companion} size={52} mood="curious" />
        </span>
      </div>
      <h1>{t("app-content-loading")}</h1>
      <p>{t("app-content-loading-body")}</p>
      <div className="boot-progress" aria-hidden="true">
        <span />
      </div>
    </main>
  );
}
