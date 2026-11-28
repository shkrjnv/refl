import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ClipboardEvent,
  type FormEvent,
  type KeyboardEvent,
} from "react";

import {
  ApiError,
  isAuthTokens,
  type AuthTokens,
  type RequestCodeResponse,
  type VerifyCodeResponse,
  type TelegramLoginPayload,
} from "../api/client";
import { AuthBackdrop } from "../components/AuthBackdrop";
import { PaletteToggle } from "../components/AppearanceMenu";
import { IellaSprite, type IellaMood } from "../components/IellaSprite";
import { LanguageMenu } from "../components/LanguageMenu";
import {
  BrandLogo,
  ConfirmationCheck,
  StudioIcon,
} from "../components/StudioIcons";
import { Button } from "../components/ui";
import { useTranslation, type Locale } from "../i18n";
import type { SkinName, ThemeName } from "../theme/tokens";
import {
  markTelegramReady,
  TELEGRAM_OAUTH_ORIGIN,
  telegramInitData,
  telegramOAuthUrl,
  telegramPayloadFromLocation,
  telegramPayloadFromMessage,
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
  theme: ThemeName;
  skin: SkinName;
  locale: Locale;
  locales: readonly Locale[];
  onLocaleChange: (locale: Locale) => void;
  onThemeChange: (theme: ThemeName) => void;
  onAuthenticated: (tokens: AuthTokens, method: AuthMethod) => void;
  onPending: (notice: PendingNotice) => void;
  onAccessBlocked: (state: "access_rejected" | "erased") => void;
}

const TELEGRAM_QUERY_KEYS = [
  "id",
  "first_name",
  "last_name",
  "username",
  "photo_url",
  "auth_date",
  "hash",
] as const;
const CODE_LENGTH = 6;
const CONFIRMATION_HOLD_MS = 650;
const REDUCED_MOTION_CONFIRMATION_HOLD_MS = 250;

export function confirmationHoldMs(): number {
  const reduced =
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  return reduced
    ? REDUCED_MOTION_CONFIRMATION_HOLD_MS
    : CONFIRMATION_HOLD_MS;
}

function clearTelegramParams(): void {
  if (typeof window === "undefined") {
    return;
  }
  const url = new URL(window.location.href);
  for (const key of TELEGRAM_QUERY_KEYS) {
    url.searchParams.delete(key);
  }
  url.hash = "";
  window.history.replaceState(null, "", `${url.pathname}${url.search}`);
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

function digitsFrom(raw: string): string[] {
  const clean = raw.replace(/\D/g, "").slice(0, CODE_LENGTH);
  return Array.from(
    { length: CODE_LENGTH },
    (_, index) => clean[index] ?? "",
  );
}

function maskEmail(email: string): string {
  const [local = "", domain] = email.split("@");
  if (!domain) {
    return email;
  }
  const visible = local.slice(0, 1);
  return `${visible}${"*".repeat(Math.max(2, local.length - 1))}@${domain}`;
}

export function AuthScreen({
  api,
  inviteCode,
  theme,
  skin,
  locale,
  locales,
  onLocaleChange,
  onThemeChange,
  onAuthenticated,
  onPending,
  onAccessBlocked,
}: AuthScreenProps) {
  const { t } = useTranslation();
  const emailId = useId();
  const codeId = useId();
  const [stage, setStage] = useState<"email" | "code">("email");
  const [email, setEmail] = useState("");
  const [digits, setDigits] = useState<string[]>(() =>
    Array.from({ length: CODE_LENGTH }, () => ""),
  );
  const [sent, setSent] = useState<RequestCodeResponse | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [telegram] = useState(() => telegramInitData());
  const [iellaMood, setIellaMood] = useState<IellaMood>("curious");
  const [thermalFocus, setThermalFocus] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [formHeight, setFormHeight] = useState<number | null>(null);
  const formRegionRef = useRef<HTMLDivElement | null>(null);
  const formContentRef = useRef<HTMLDivElement | null>(null);
  const handledReturnRef = useRef(false);
  const otpRefs = useRef<Array<HTMLInputElement | null>>([]);
  const confirmationTimerRef = useRef<number | null>(null);
  const callbacksRef = useRef({ onAuthenticated, onPending, onAccessBlocked });
  callbacksRef.current = { onAuthenticated, onPending, onAccessBlocked };
  const code = digits.join("");
  const maskedEmail = maskEmail(email.trim());

  useLayoutEffect(() => {
    if (confirmed) {
      return;
    }
    const region = formRegionRef.current;
    const content = formContentRef.current;
    if (!region || !content || typeof ResizeObserver === "undefined") {
      return;
    }
    const measure = () => {
      setFormHeight(content.scrollHeight);
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(content);
    return () => {
      observer.disconnect();
    };
  }, [stage, error, sent, inviteCode, confirmed]);

  const completeAuthentication = (tokens: AuthTokens, method: AuthMethod) => {
    setConfirmed(true);
    if (confirmationTimerRef.current !== null) {
      window.clearTimeout(confirmationTimerRef.current);
    }
    confirmationTimerRef.current = window.setTimeout(() => {
      confirmationTimerRef.current = null;
      callbacksRef.current.onAuthenticated(tokens, method);
    }, confirmationHoldMs());
  };

  const handleResponse = (response: VerifyCodeResponse, method: AuthMethod) => {
    clearTelegramParams();
    if (isAuthTokens(response)) {
      completeAuthentication(response, method);
      return;
    }
    callbacksRef.current.onPending({
      learnerId: response.learner_id,
      method,
    });
  };

  const authenticateTelegram = useCallback(
    async (payload: TelegramLoginPayload) => {
      setBusy(true);
      setError(null);
      setIellaMood("curious");
      try {
        const response = await api.telegramLogin(
          payload,
          typeof window === "undefined" ? "" : window.location.search,
        );
        handleResponse(response, "telegram");
      } catch (caught) {
        if (caught instanceof ApiError) {
          if (caught.code === "access_rejected" || caught.code === "erased") {
            callbacksRef.current.onAccessBlocked(caught.code);
            return;
          }
        }
        setIellaMood("proud");
        setError(authErrorKey(caught));
      } finally {
        setBusy(false);
      }
    },
    [api],
  );

  const consumeTelegramReturn = useCallback(() => {
    if (typeof window === "undefined" || handledReturnRef.current) {
      return;
    }
    const payload = telegramPayloadFromLocation(
      window.location.search,
      window.location.hash,
    );
    if (!payload) {
      return;
    }
    handledReturnRef.current = true;
    clearTelegramParams();
    void authenticateTelegram(payload);
  }, [authenticateTelegram]);

  useEffect(() => {
    markTelegramReady();
    if (typeof window === "undefined") {
      return;
    }
    const consume = () => consumeTelegramReturn();
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        consume();
      }
    };
    const onMessage = (event: MessageEvent) => {
      if (event.origin !== TELEGRAM_OAUTH_ORIGIN || handledReturnRef.current) {
        return;
      }
      const payload = telegramPayloadFromMessage(event.data);
      if (!payload) {
        return;
      }
      handledReturnRef.current = true;
      clearTelegramParams();
      void authenticateTelegram(payload);
    };
    consume();
    window.addEventListener("hashchange", consume);
    window.addEventListener("focus", consume);
    window.addEventListener("message", onMessage);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      window.removeEventListener("hashchange", consume);
      window.removeEventListener("focus", consume);
      window.removeEventListener("message", onMessage);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [authenticateTelegram, consumeTelegramReturn]);

  useEffect(
    () => () => {
      if (confirmationTimerRef.current !== null) {
        window.clearTimeout(confirmationTimerRef.current);
      }
    },
    [],
  );

  const requestCode = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setIellaMood("curious");
    try {
      const response = await api.requestCode(email.trim());
      setSent(response);
      setDigits(Array.from({ length: CODE_LENGTH }, () => ""));
      setStage("code");
      window.setTimeout(() => otpRefs.current[0]?.focus(), 140);
    } catch (caught) {
      setError(authErrorKey(caught));
    } finally {
      setBusy(false);
    }
  };

  const verifyCode = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (code.length !== CODE_LENGTH) {
      return;
    }
    setBusy(true);
    setError(null);
    setIellaMood("curious");
    try {
      const response = await api.verifyCode(
        email.trim(),
        code,
        inviteCode,
      );
      handleResponse(response, "email");
    } catch (caught) {
      if (caught instanceof ApiError) {
        if (caught.code === "access_rejected" || caught.code === "erased") {
          callbacksRef.current.onAccessBlocked(caught.code);
          return;
        }
      }
      setIellaMood("curious");
      setError(authErrorKey(caught));
    } finally {
      setBusy(false);
    }
  };

  const updateDigit = (index: number, raw: string) => {
    if (raw.length > 1) {
      const next = digitsFrom(raw);
      setDigits(next);
      const filled = next.findIndex((digit) => digit === "");
      const focusIndex = filled < 0 ? CODE_LENGTH - 1 : filled;
      otpRefs.current[focusIndex]?.focus();
      return;
    }
    const digit = raw.replace(/\D/g, "").slice(0, 1);
    setDigits((current) =>
      current.map((value, position) => (position === index ? digit : value)),
    );
    if (digit && index < CODE_LENGTH - 1) {
      otpRefs.current[index + 1]?.focus();
    }
  };

  const onDigitKeyDown = (
    index: number,
    event: KeyboardEvent<HTMLInputElement>,
  ) => {
    if (event.key === "Backspace" && digits[index] === "" && index > 0) {
      event.preventDefault();
      otpRefs.current[index - 1]?.focus();
      return;
    }
    if (event.key === "ArrowLeft" && index > 0) {
      event.preventDefault();
      otpRefs.current[index - 1]?.focus();
    }
    if (event.key === "ArrowRight" && index < CODE_LENGTH - 1) {
      event.preventDefault();
      otpRefs.current[index + 1]?.focus();
    }
  };

  const onPasteCode = (event: ClipboardEvent<HTMLDivElement>) => {
    const raw = event.clipboardData.getData("text");
    if (!raw) {
      return;
    }
    event.preventDefault();
    const next = digitsFrom(raw);
    setDigits(next);
    const filled = next.findIndex((digit) => digit === "");
    otpRefs.current[filled < 0 ? CODE_LENGTH - 1 : filled]?.focus();
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
    if (typeof window === "undefined") {
      setError("auth-telegram-unavailable");
      return;
    }
    const returnTo = `${window.location.origin}${window.location.pathname}${window.location.search}`;
    const url = telegramOAuthUrl(botId, window.location.origin, returnTo);
    if (!url) {
      setError("auth-telegram-unavailable");
      return;
    }
    window.location.assign(url);
  };

  const backToEmail = () => {
    setStage("email");
    setDigits(Array.from({ length: CODE_LENGTH }, () => ""));
    setError(null);
    setIellaMood("curious");
  };

  return (
    <div className="auth-screen" data-skin={skin}>
      <section className="auth-panel" aria-label={t("auth-title")}>
        <div className="auth-panel-topbar">
          <div className="auth-brand-group">
            <div className="auth-brand">
              <BrandLogo size={40} />
              <span className="brand-name">{t("app-name")}</span>
            </div>
            <LanguageMenu
              locale={locale}
              locales={locales}
              onLocaleChange={onLocaleChange}
            />
          </div>
          <div className="auth-header-controls">
            <PaletteToggle theme={theme} onThemeChange={onThemeChange} />
          </div>
        </div>

        <div
          ref={formRegionRef}
          className={`auth-form-region auth-stage-${stage}${
            formHeight !== null ? " auth-form-region-sized" : ""
          }`}
          style={formHeight !== null ? { height: formHeight } : undefined}
        >
          {confirmed ? (
            <div className="auth-form-inner auth-form-inner-confirmed">
              <div className="auth-confirm-state" role="status" aria-live="polite">
                <div className="auth-confirm-mark" aria-hidden="true">
                  <ConfirmationCheck size={32} />
                </div>
                <p>{t("auth-signed-in")}</p>
              </div>
            </div>
          ) : (
            <div ref={formContentRef} className="auth-form-inner">
              <div className="auth-card-head auth-form-header">
                <div>
                  <h2 className="auth-form-title">
                    {stage === "code" ? t("auth-code-title") : t("auth-title")}
                  </h2>
                  {stage === "code" ? (
                    <p className="auth-card-subtitle">
                      {t("auth-code-target", { target: maskedEmail })}
                    </p>
                  ) : null}
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
                <form
                  className="auth-form auth-form-email"
                  onSubmit={(event) => void requestCode(event)}
                >
                  <label htmlFor={emailId}>{t("auth-email-label")}</label>
                  <input
                    id={emailId}
                    type="email"
                    autoComplete="email"
                    value={email}
                    onChange={(event) => setEmail(event.target.value)}
                    onFocus={() => setThermalFocus({ x: 0.18, y: 0.58 })}
                    onBlur={() => setThermalFocus(null)}
                    placeholder={t("auth-email-placeholder")}
                    required
                  />
                  <Button type="submit" variant="primary" block busy={busy}>
                    <StudioIcon name="arrow-right" size={16} />
                    <span className="auth-submit-label">{t("auth-send-code")}</span>
                  </Button>
                </form>
              ) : (
                <form
                  className="auth-form auth-form-code"
                  onSubmit={(event) => void verifyCode(event)}
                >
                  <label className="visually-hidden" htmlFor={codeId}>
                    {t("auth-code-label")}
                  </label>
                  <div
                    className="auth-code-slots"
                    role="group"
                    aria-label={t("auth-code-group")}
                    onPaste={onPasteCode}
                  >
                    {digits.map((digit, index) => (
                      <input
                        key={index}
                        ref={(element) => {
                          otpRefs.current[index] = element;
                        }}
                        id={index === 0 ? codeId : undefined}
                        className="auth-code-slot"
                        inputMode="numeric"
                        pattern="[0-9]*"
                        autoComplete={index === 0 ? "one-time-code" : "off"}
                        maxLength={1}
                        value={digit}
                        aria-label={
                          index === 0
                            ? undefined
                            : t("auth-code-digit", { index: index + 1 })
                        }
                        onChange={(event) => updateDigit(index, event.target.value)}
                        onFocus={() => setThermalFocus({ x: 0.12, y: 0.46 })}
                        onBlur={() => setThermalFocus(null)}
                        onKeyDown={(event) => onDigitKeyDown(index, event)}
                      />
                    ))}
                  </div>
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
                  <Button
                    type="submit"
                    variant="primary"
                    block
                    busy={busy}
                    disabled={code.length !== CODE_LENGTH}
                  >
                    <StudioIcon name="check" size={16} />
                    <span className="auth-submit-label">{t("auth-verify-code")}</span>
                  </Button>
                  <Button type="button" variant="ghost" block onClick={backToEmail}>
                    <StudioIcon name="arrow-left" size={16} />
                    {t("auth-change-email")}
                  </Button>
                </form>
              )}

              <div className="auth-divider">
                <span>{t("auth-or")}</span>
              </div>
              <Button
                type="button"
                variant="secondary"
                block
                busy={busy}
                onClick={startTelegram}
              >
                <StudioIcon name="arrow-right" size={16} />
                <span className="auth-submit-label">{t("auth-telegram")}</span>
              </Button>
              {telegram ? (
                <p className="auth-note">{t("auth-telegram-detected")}</p>
              ) : null}
              <p className="auth-legal">{t("auth-invite-only")}</p>
            </div>
          )}
        </div>
      </section>

      <section className="auth-visual" aria-label={t("auth-welcome")}>
        <AuthBackdrop focus={thermalFocus} />
        <div className="auth-studio-grain" aria-hidden="true" />
        <div className="auth-visual-copy">
          <h1>{t("auth-welcome-title")}</h1>
          <p>{t("auth-welcome-body")}</p>
        </div>
        <div className="auth-iella-scene">
          <IellaSprite
            size={68}
            mood={iellaMood}
            label={t("auth-iella-hello")}
            onActivate={() =>
              setIellaMood((current) =>
                current === "proud" ? "curious" : "proud",
              )
            }
          />
        </div>
      </section>
    </div>
  );
}
