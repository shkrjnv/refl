import { useEffect, useId, useRef } from "react";

import type { TelegramLoginPayload } from "../api/client";
import { telegramPayloadFromMessage } from "../prototype/telegram";

export interface TelegramLoginWidgetProps {
  username: string;
  onAuth: (payload: TelegramLoginPayload) => void;
  onError: () => void;
  className?: string;
}

const BOT_USERNAME = /^[A-Za-z0-9_]{3,}$/;

/**
 * The official Telegram Login Widget. It uses the bot username from
 * `VITE_TELEGRAM_BOT_USERNAME`; the returned hash is verified by the server
 * with the Login Widget secret.
 */
export function TelegramLoginWidget({
  username,
  onAuth,
  onError,
  className = "",
}: TelegramLoginWidgetProps) {
  const rawId = useId();
  const containerRef = useRef<HTMLDivElement | null>(null);
  const onAuthRef = useRef(onAuth);
  const onErrorRef = useRef(onError);
  onAuthRef.current = onAuth;
  onErrorRef.current = onError;

  useEffect(() => {
    const container = containerRef.current;
    const bot = username.trim();
    if (!container || !BOT_USERNAME.test(bot)) {
      onErrorRef.current();
      return;
    }
    const callbackName = `__reflTelegramAuth_${rawId.replace(/[^A-Za-z0-9_]/g, "")}`;
    const globals = window as unknown as Record<string, unknown>;
    globals[callbackName] = (user: unknown) => {
      const payload = telegramPayloadFromMessage(user);
      if (payload) {
        onAuthRef.current(payload);
      } else {
        onErrorRef.current();
      }
    };
    const script = document.createElement("script");
    script.src = "https://telegram.org/js/telegram-widget.js?22";
    script.async = true;
    script.setAttribute("data-telegram-login", bot);
    script.setAttribute("data-size", "large");
    script.setAttribute("data-userpic", "false");
    script.setAttribute("data-onauth", `${callbackName}(user)`);
    script.onerror = () => onErrorRef.current();
    container.replaceChildren(script);
    return () => {
      script.remove();
      delete globals[callbackName];
    };
  }, [rawId, username]);

  return (
    <div
      ref={containerRef}
      className={`telegram-login-widget ${className}`.trim()}
      data-telegram-username={username}
    />
  );
}
