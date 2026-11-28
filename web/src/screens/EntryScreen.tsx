import { IellaSprite } from "../components/IellaSprite";
import { Button, Card } from "../components/ui";
import { useTranslation } from "../i18n";

export interface EntryScreenProps {
  placementAvailable: boolean;
  busy: boolean;
  error: string | null;
  onStartFromZero: () => void;
  onCheck: () => void;
}

export function EntryScreen({
  placementAvailable,
  busy,
  error,
  onStartFromZero,
  onCheck,
}: EntryScreenProps) {
  const { t } = useTranslation();
  return (
    <div className="access-screen">
      <Card tone="plain" className="access-card">
        <div className="access-emblem">
          <IellaSprite size={88} />
        </div>
        <p className="eyebrow">{t("entry-eyebrow")}</p>
        <h1>{t("entry-title")}</h1>
        <p className="access-body">{t("entry-body")}</p>
        <div className="access-actions">
          <Button
            type="button"
            variant="primary"
            block
            busy={busy}
            onClick={onStartFromZero}
          >
            {t("entry-zero-action")}
          </Button>
          <p className="access-note">{t("entry-zero-body")}</p>
          <Button
            type="button"
            variant="secondary"
            block
            disabled={!placementAvailable || busy}
            onClick={onCheck}
          >
            {t("entry-placement-action")}
          </Button>
          <p className="access-note">
            {placementAvailable
              ? t("entry-placement-body")
              : t("entry-placement-empty")}
          </p>
        </div>
        {error ? (
          <p className="inline-alert danger" role="alert">
            {error}
          </p>
        ) : null}
      </Card>
    </div>
  );
}
