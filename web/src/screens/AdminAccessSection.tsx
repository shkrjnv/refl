import { useEffect, useState } from "react";

import type {
  ApiClient,
  CreatedInvite,
  PendingUser,
} from "../api/client";
import { Button, Card } from "../components/ui";
import { CrossfadeIcon } from "../components/StudioIcons";
import { useTranslation } from "../i18n";

type AdminApi = Pick<
  ApiClient,
  "createInvite" | "pendingRequests" | "approve" | "reject"
>;

export interface AdminAccessSectionProps {
  api: AdminApi;
}

function inviteUrl(code: string): string {
  if (typeof window === "undefined") {
    return `/?invite=${encodeURIComponent(code)}`;
  }
  return `${window.location.origin}/?invite=${encodeURIComponent(code)}`;
}

export function AdminAccessSection({ api }: AdminAccessSectionProps) {
  const { t } = useTranslation();
  const [note, setNote] = useState("");
  const [maxUses, setMaxUses] = useState(1);
  const [invite, setInvite] = useState<CreatedInvite | null>(null);
  const [pending, setPending] = useState<PendingUser[]>([]);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadPending = async () => {
    try {
      setPending(await api.pendingRequests());
    } catch {
      setError(t("admin-load-failed"));
    }
  };

  useEffect(() => {
    void loadPending();
  }, []);

  const create = async () => {
    setBusy(true);
    setError(null);
    setCopied(false);
    try {
      setInvite(
        await api.createInvite({
          note: note.trim() || undefined,
          max_uses: Math.max(1, maxUses),
        }),
      );
    } catch {
      setError(t("admin-invite-failed"));
    } finally {
      setBusy(false);
    }
  };

  const copy = async () => {
    if (!invite) {
      return;
    }
    try {
      await navigator.clipboard.writeText(inviteUrl(invite.invite_code));
      setCopied(true);
    } catch {
      setError(t("admin-copy-failed"));
    }
  };

  const decide = async (user: PendingUser, approve: boolean) => {
    setBusy(true);
    setError(null);
    try {
      if (approve) {
        await api.approve(user.user_id);
      } else {
        await api.reject(user.user_id);
      }
      await loadPending();
    } catch {
      setError(t("admin-decision-failed"));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card tone="plain" className="admin-section">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("admin-eyebrow")}</p>
          <h2>{t("admin-title")}</h2>
        </div>
      </div>
      <p className="admin-intro">{t("admin-intro")}</p>

      {error ? (
        <p className="inline-alert danger" role="alert">
          {error}
        </p>
      ) : null}

      <div className="admin-create">
        <label>
          {t("admin-note-label")}
          <input
            value={note}
            onChange={(event) => setNote(event.target.value)}
            placeholder={t("admin-note-placeholder")}
          />
        </label>
        <label>
          {t("admin-uses-label")}
          <input
            type="number"
            min={1}
            max={999}
            value={maxUses}
            onChange={(event) =>
              setMaxUses(Number(event.target.value) || 1)
            }
          />
        </label>
        <Button
          type="button"
          variant="primary"
          block
          busy={busy}
          onClick={() => void create()}
        >
          {t("admin-create-invite")}
        </Button>
      </div>

      {invite ? (
        <div className="invite-result">
          <p className="eyebrow">{t("admin-invite-ready")}</p>
          <code>{invite.invite_code}</code>
          <Button type="button" variant="secondary" size="sm" onClick={() => void copy()}>
            <CrossfadeIcon
              name="copy"
              activeName="check"
              active={copied}
              size={16}
            />
            {copied ? t("admin-copied") : t("admin-copy-link")}
          </Button>
          <p>{inviteUrl(invite.invite_code)}</p>
        </div>
      ) : null}

      <div className="pending-head">
        <h3>{t("admin-pending")}</h3>
        <Button type="button" variant="ghost" size="sm" onClick={() => void loadPending()}>
          {t("common-retry")}
        </Button>
      </div>
      <div className="pending-list">
        {pending.length === 0 ? (
          <p className="empty-note">{t("admin-pending-empty")}</p>
        ) : (
          pending.map((user) => (
            <div className="pending-row" key={user.user_id}>
              <div>
                <strong>{user.display_name}</strong>
                <span>
                  {user.login_methods
                    .map((method) => `${method.kind}: ${method.identifier}`)
                    .join(" · ") || t("admin-no-method")}
                </span>
              </div>
              <div className="pending-actions">
                <Button
                  type="button"
                  variant="primary"
                  size="sm"
                  busy={busy}
                  onClick={() => void decide(user, true)}
                >
                  {t("admin-approve")}
                </Button>
                <Button
                  type="button"
                  variant="danger"
                  size="sm"
                  busy={busy}
                  onClick={() => void decide(user, false)}
                >
                  {t("admin-reject")}
                </Button>
              </div>
            </div>
          ))
        )}
      </div>
    </Card>
  );
}
