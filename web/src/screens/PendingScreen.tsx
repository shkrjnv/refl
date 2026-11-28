import { IellaSprite } from "../components/IellaSprite";
import { Button, Card } from "../components/ui";
import { useTranslation } from "../i18n";

export type AccessState = "pending_approval" | "access_rejected" | "erased";

export interface PendingScreenProps {
  state: AccessState;
  onCheck: () => void;
  onClose?: () => void;
}

export function PendingScreen({ state, onCheck, onClose }: PendingScreenProps) {
  const { t } = useTranslation();
  const title =
    state === "pending_approval"
      ? "auth-pending-title"
      : state === "access_rejected"
        ? "auth-rejected-title"
        : "auth-erased-title";
  const body =
    state === "pending_approval"
      ? "auth-pending-body"
      : state === "access_rejected"
        ? "auth-rejected-body"
        : "auth-erased-body";
  const tone =
    state === "pending_approval"
      ? "sun"
      : state === "access_rejected"
        ? "coral"
        : "neutral";

  return (
    <div className="access-screen">
      <Card tone="plain" className="access-card">
        <div className={`access-emblem access-emblem-${tone}`}>
          <IellaSprite size={88} />
        </div>
        <p className="eyebrow">{t("auth-access-eyebrow")}</p>
        <h1>{t(title)}</h1>
        <p className="access-body">{t(body)}</p>
        <div className="access-actions">
          {state === "pending_approval" ? (
            <Button variant="primary" block onClick={onCheck}>
              {t("auth-check-status")}
            </Button>
          ) : (
            <Button variant="secondary" block onClick={onCheck}>
              {t("auth-back-signin")}
            </Button>
          )}
          {onClose ? (
            <Button variant="ghost" block onClick={onClose}>
              {t("common-close")}
            </Button>
          ) : null}
        </div>
        {state === "pending_approval" ? (
          <p className="access-note">{t("auth-pending-check-note")}</p>
        ) : null}
      </Card>
    </div>
  );
}
