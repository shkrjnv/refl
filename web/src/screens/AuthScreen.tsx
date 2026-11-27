import { useEffect, useId, useState, type FormEvent } from "react";

import {
  ApiError,
  isAuthTokens,
  type AuthTokens,
  type RequestCodeResponse,
  type VerifyCodeResponse,
  type TelegramLoginPayload,
} from "../api/client";
import { HeroIllustration } from "../components/HeroIllustration";
import { IellaSprite } from "../components/IellaSprite";
import { Button, Card } from "../components/ui";
import { useTranslation } from "../i18n";
import {
  markTelegramReady,
  telegramInitData,
  telegramOAuthUrl,
  telegramPayloadFromWebApp,
} from "../prototype/telegram";

export interface AuthApi {
  requestCode(email: string): Promise<RequestCodeResponse>;
  verifyCode(
    email: string,
    code: string,
    inviteCode?: string | null,
  ): Promise<VerifyCodeResponse>;
  telegramLogin(
    payload: TelegramLoginPayload,
    search?: string,
  ): Promise<VerifyCodeResponse>;
}

export type AuthMethod = "email" | "telegram";

export interface PendingNotice {
  learnerId: string;
  method: AuthMethod;
}

export interface AuthScreenProps {
  api: AuthApi;
  inviteCode: string | null;
  onAuthenticated: (tokens: AuthTokens, method: AuthMethod) => void;
  onPending: (notice: PendingNotice) => void;
  onAccessBlocked: (state: "access_rejected" | "erased") => void;
}

function telegramPayloadFromSearch(search: string): TelegramLoginPayload | null {
  const params = new URLSearchParams(search);
  const idRaw = params.get("id");
  const firstName = params.get("first_name");
  const authDateRaw = params.get("auth_date");
  const hash = params.get("hash");
  if (!idRaw || !firstName || !authDateRaw || !hash) {
    return null;
  }
  const id = Number(idRaw);
  const authDate = Number(authDateRaw);
  if (!Number.isFinite(id) || !Number.isFinite(authDate)) {
    return null;
  }
  return {
    id,
    first_name: firstName,
    last_name: params.get("last_name"),
    username: params.get("username"),
    photo_url: params.get("photo_url"),
    auth_date: authDate,
    hash,
  };
}

function authErrorKey(error: unknown): string {
  if (error instanceof ApiError) {
    switch (error.code) {
      case "rate_limited":
        return "auth-rate-limited";
      case "pending_approval":
        return "auth-pending-title";
      case "access_rejected":
        return "auth-rejected-title";
      case "erased":
        return "auth-erased-title";
      case "invite_invalid":
        return "auth-invite-invalid";
      case "service_unavailable":
        return "auth-telegram-unavailable";
      case "unauthorized":
        return "auth-invalid-code";
      default:
        return "common-unknown-error";
    }
  }
  return "common-unknown-error";
}

export function AuthScreen({
  api,
  inviteCode,
  onAuthenticated,
  onPending,
  onAccessBlocked,
}: AuthScreenProps) {
  const { t } = useTranslation();
  const emailId = useId();
  const codeId = useId();
  const [stage, setStage] = useState<"email" | "code">("email");
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [sent, setSent] = useState<RequestCodeResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [telegram] = useState(() => telegramInitData());

  const clearTelegramParams = () => {
    if (typeof window === "undefined") {
      return;
    }
    const url = new URL(window.location.href);
    for (const key of [
      "id",
      "first_name",
      "last_name",
      "username",
      "photo_url",
      "auth_date",
      "hash",
    ]) {
      url.searchParams.delete(key);
    }
    url.hash = "";
    window.history.replaceState(
      null,
      "",
      `${url.pathname}${url.search}`,
    );
  };

  const handleResponse = (response: VerifyCodeResponse, method: AuthMethod) => {
    if (isAuthTokens(response)) {
      if (typeof window !== "undefined") {
        window.history.replaceState(null, "", window.location.pathname);
      }
      onAuthenticated(response, method);
      return;
    }
    clearTelegramParams();
    onPending({ learnerId: response.learner_id, method });
  };

  const authenticateTelegram = async (payload: TelegramLoginPayload) => {
    setBusy(true);
    setError(null);
    try {
      const response = await api.telegramLogin(
        payload,
        typeof window === "undefined" ? "" : window.location.search,
      );
      handleResponse(response, "telegram");
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.code === "access_rejected" || caught.code === "erased") {
          onAccessBlocked(caught.code);
          return;
        }
      }
      setError(authErrorKey(caught));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    markTelegramReady();
    if (typeof window === "undefined") {
      return;
    }
    const payload = telegramPayloadFromSearch(window.location.search);
    if (payload) {
      void authenticateTelegram(payload);
    }
    // The Telegram return URL is handled once per mount. WebApp initData is
    // used by the explicit Telegram button below.
  }, []);

  const requestCode = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const response = await api.requestCode(email.trim());
      setSent(response);
      setStage("code");
    } catch (caught) {
      setError(authErrorKey(caught));
    } finally {
      setBusy(false);
    }
  };

  const verifyCode = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const response = await api.verifyCode(
        email.trim(),
        code.trim(),
        inviteCode,
      );
      handleResponse(response, "email");
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.code === "access_rejected" || caught.code === "erased") {
          onAccessBlocked(caught.code);
          return;
        }
      }
      setError(authErrorKey(caught));
    } finally {
      setBusy(false);
    }
  };

  const startTelegram = () => {
    const webAppPayload = telegram
      ? telegramPayloadFromWebApp(telegram)
      : null;
    if (webAppPayload) {
      void authenticateTelegram(webAppPayload);
      return;
    }
    const botId = import.meta.env.VITE_TELEGRAM_BOT_ID ?? "";
    const returnTo =
      typeof window === "undefined" ? "" : window.location.href;
    const url =
      typeof window === "undefined"
        ? null
        : telegramOAuthUrl(botId, window.location.origin, returnTo);
    if (!url) {
      setError("auth-telegram-unavailable");
      return;
    }
    window.open(url, "_blank", "noopener,noreferrer");
  };

  return (
    <div className="auth-screen">
      <section className="auth-visual" aria-label={t("auth-welcome")}>
        <div className="auth-illustration">
          <HeroIllustration />
        </div>
        <div className="auth-visual-copy">
          <div className="iella-arrival">
            <IellaSprite size={76} />
          </div>
          <p className="eyebrow">{t("auth-welcome-eyebrow")}</p>
          <h1>{t("auth-welcome-title")}</h1>
          <p>{t("auth-welcome-body")}</p>
        </div>
      </section>

      <section className="auth-panel">
        <Card tone="plain" className="auth-card">
          <div className="auth-card-head">
            <span className="brand-mark" aria-hidden="true">
              r
            </span>
            <div>
              <p className="eyebrow">{t("auth-title")}</p>
              <h2>{t("auth-subtitle")}</h2>
            </div>
          </div>

          {inviteCode ? (
            <div className="invite-banner">
              <span>{t("auth-invite-label")}</span>
              <code>{inviteCode}</code>
            </div>
          ) : null}

          {error ? (
            <p className="inline-alert danger" role="alert">
              {t(error)}
            </p>
          ) : null}

          {stage === "email" ? (
            <form className="auth-form" onSubmit={(event) => void requestCode(event)}>
              <label htmlFor={emailId}>{t("auth-email-label")}</label>
              <input
                id={emailId}
                type="email"
                autoComplete="email"
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                placeholder={t("auth-email-placeholder")}
                required
              />
              <Button type="submit" variant="primary" block busy={busy}>
                {t("auth-send-code")}
              </Button>
            </form>
          ) : (
            <form className="auth-form" onSubmit={(event) => void verifyCode(event)}>
              <label htmlFor={codeId}>{t("auth-code-label")}</label>
              <input
                id={codeId}
                className="code-input"
                inputMode="numeric"
                autoComplete="one-time-code"
                maxLength={6}
                value={code}
                onChange={(event) =>
                  setCode(event.target.value.replace(/\D/g, "").slice(0, 6))
                }
                required
              />
              {sent ? (
                <p className="auth-note">
                  {t("auth-code-sent", {
                    minutes: Math.max(1, Math.round(sent.expires_in / 60)),
                  })}
                </p>
              ) : null}
              {sent?.code || sent?.dev_code ? (
                <p className="dev-code" role="status">
                  {t("auth-dev-code", { code: sent.code ?? sent.dev_code ?? "" })}
                </p>
              ) : null}
              <Button type="submit" variant="primary" block busy={busy}>
                {t("auth-verify-code")}
              </Button>
              <Button
                type="button"
                variant="ghost"
                block
                onClick={() => {
                  setStage("email");
                  setCode("");
                  setError(null);
                }}
              >
                {t("auth-change-email")}
              </Button>
            </form>
          )}

          <div className="auth-divider">
            <span>{t("auth-or")}</span>
          </div>
          <Button type="button" variant="secondary" block busy={busy} onClick={startTelegram}>
            {t("auth-telegram")}
          </Button>
          {telegram ? (
            <p className="auth-note">{t("auth-telegram-detected")}</p>
          ) : null}
          <p className="auth-legal">{t("auth-invite-only")}</p>
        </Card>
      </section>
    </div>
  );
}
