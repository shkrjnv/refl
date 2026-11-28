import { useTranslation } from "../i18n";
import {
  COMPANIONS,
  type CompanionId,
} from "../companions/registry";
import { CompanionSprite } from "./CompanionSprite";
import { Button, StatusMeta } from "./ui";

export interface CompanionPickerProps {
  companion: CompanionId;
  sendoUnlocked: boolean;
  sendoPreview: boolean;
  onSelect: (companion: CompanionId) => void;
  onPreviewSendo: () => void;
}

/** Companion registry card used by Profile and Settings. */
export function CompanionPicker({
  companion,
  sendoUnlocked,
  sendoPreview,
  onSelect,
  onPreviewSendo,
}: CompanionPickerProps) {
  const { t } = useTranslation();
  return (
    <section className="companion-picker" aria-labelledby="companion-picker-title">
      <div className="companion-picker-head">
        <div>
          <p className="eyebrow">{t("companion-eyebrow")}</p>
          <h2 id="companion-picker-title">{t("companion-picker-title")}</h2>
        </div>
        <span className="bento-note">{t("companion-picker-note")}</span>
      </div>
      <div className="companion-grid">
        {COMPANIONS.map((entry) => {
          const unlocked = entry.base || sendoUnlocked;
          const selected = companion === entry.id;
          const previewing = entry.id === "sendo" && sendoPreview;
          return (
            <article
              key={entry.id}
              className={`companion-card${selected ? " selected" : ""}`}
              data-companion={entry.id}
            >
              <div className="companion-art">
                <CompanionSprite
                  companion={entry.id}
                  size={64}
                  mood={entry.id === "sendo" && previewing ? "surprised" : "idle"}
                />
              </div>
              <div className="companion-copy">
                <span className="bento-kicker">
                  {selected ? t("companion-yours") : t(entry.roleKey)}
                </span>
                <h3>{t(entry.nameKey)}</h3>
                <p>{t(entry.descriptionKey)}</p>
                <StatusMeta tone={unlocked ? "success" : "neutral"}>
                  {unlocked ? t("companion-unlocked") : t(entry.unlockKey)}
                </StatusMeta>
              </div>
              <div className="companion-actions">
                {unlocked ? (
                  <Button
                    type="button"
                    variant={selected ? "primary" : "secondary"}
                    size="sm"
                    aria-pressed={selected}
                    onClick={() => onSelect(entry.id)}
                  >
                    {selected ? t("companion-selected") : t("companion-select")}
                  </Button>
                ) : (
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    aria-pressed={previewing}
                    onClick={onPreviewSendo}
                  >
                    {previewing ? t("companion-preview-hide") : t("companion-preview")}
                  </Button>
                )}
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}
