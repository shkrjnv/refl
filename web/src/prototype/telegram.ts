import type { TelegramLoginPayload } from "../api/client";
import { isJsonObject } from "../types/json";

interface TelegramWebApp {
  initData?: string;
  ready?: () => void;
}

declare global {
  interface Window {
    Telegram?: {
      WebApp?: TelegramWebApp;
    };
  }
}

export function telegramWebApp(): TelegramWebApp | null {
  return typeof window === "undefined" ? null : (window.Telegram?.WebApp ?? null);
}

export function telegramInitData(): string | null {
  const data = telegramWebApp()?.initData?.trim();
  return data ? data : null;
}

export function markTelegramReady(): void {
  telegramWebApp()?.ready?.();
}

export function telegramOAuthUrl(
  botId: string,
  origin: string,
  returnTo: string,
): string | null {
  if (!botId) {
    return null;
  }
  const url = new URL("https://oauth.telegram.org/auth");
  url.searchParams.set("bot_id", botId);
  url.searchParams.set("origin", origin);
  url.searchParams.set("return_to", returnTo);
  return url.toString();
}

export function telegramPayloadFromWebApp(
  initData: string,
): TelegramLoginPayload | null {
  const params = new URLSearchParams(initData);
  const userRaw = params.get("user");
  const authDateRaw = params.get("auth_date");
  const hash = params.get("hash");
  if (!userRaw || !authDateRaw || !hash) {
    return null;
  }
  const authDate = Number(authDateRaw);
  if (!Number.isFinite(authDate)) {
    return null;
  }
  let user: unknown;
  try {
    user = JSON.parse(userRaw) as unknown;
  } catch {
    return null;
  }
  if (!isJsonObject(user)) {
    return null;
  }
  const id = user["id"];
  const firstName = user["first_name"];
  if (typeof id !== "number" || typeof firstName !== "string") {
    return null;
  }
  const lastName = user["last_name"];
  const username = user["username"];
  const photoUrl = user["photo_url"];
  return {
    id,
    first_name: firstName,
    last_name: typeof lastName === "string" ? lastName : null,
    username: typeof username === "string" ? username : null,
    photo_url: typeof photoUrl === "string" ? photoUrl : null,
    auth_date: authDate,
    hash,
  };
}
