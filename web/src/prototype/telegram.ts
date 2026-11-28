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
    init_data: initData,
  };
}

export const TELEGRAM_OAUTH_ORIGIN = "https://oauth.telegram.org";

function stringValue(value: unknown): string | null {
  return typeof value === "string" && value.trim().length > 0
    ? value.trim()
    : null;
}

function numberValue(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim().length > 0) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function payloadFromRecord(value: unknown): TelegramLoginPayload | null {
  if (!isJsonObject(value)) {
    return null;
  }
  const id = numberValue(value["id"]);
  const firstName = stringValue(value["first_name"]);
  const authDate = numberValue(value["auth_date"]);
  const hash = stringValue(value["hash"]);
  if (id === null || firstName === null || authDate === null || hash === null) {
    return null;
  }
  return {
    id,
    first_name: firstName,
    last_name: stringValue(value["last_name"]),
    username: stringValue(value["username"]),
    photo_url: stringValue(value["photo_url"]),
    auth_date: authDate,
    hash,
  };
}

export function telegramPayloadFromSearch(
  search: string,
): TelegramLoginPayload | null {
  const params = new URLSearchParams(search);
  return payloadFromRecord({
    id: params.get("id"),
    first_name: params.get("first_name"),
    last_name: params.get("last_name"),
    username: params.get("username"),
    photo_url: params.get("photo_url"),
    auth_date: params.get("auth_date"),
    hash: params.get("hash"),
  });
}

function decodeBase64(value: string): string | null {
  const normalized = value
    .replace(/-/g, "+")
    .replace(/_/g, "/")
    .replace(/\s+/g, "");
  if (!normalized || typeof atob !== "function") {
    return null;
  }
  const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
  try {
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (character) =>
      character.charCodeAt(0),
    );
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}

export function telegramPayloadFromAuthResult(
  value: string,
): TelegramLoginPayload | null {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  let candidate = trimmed;
  if (trimmed.includes("%")) {
    try {
      candidate = decodeURIComponent(trimmed);
    } catch {
      candidate = trimmed;
    }
  }
  const json = candidate.startsWith("{") ? candidate : decodeBase64(candidate);
  if (!json) {
    return null;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(json) as unknown;
  } catch {
    return null;
  }
  return payloadFromRecord(parsed);
}

function authResultFromHash(hash: string): string | null {
  const raw = hash.startsWith("#") ? hash.slice(1) : hash;
  const marker = "tgAuthResult=";
  const index = raw.indexOf(marker);
  if (index < 0) {
    return null;
  }
  const rest = raw.slice(index + marker.length);
  const end = rest.indexOf("&");
  const value = end >= 0 ? rest.slice(0, end) : rest;
  if (!value) {
    return null;
  }
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export function telegramPayloadFromHash(
  hash: string,
): TelegramLoginPayload | null {
  const value = authResultFromHash(hash);
  return value ? telegramPayloadFromAuthResult(value) : null;
}

export function telegramPayloadFromLocation(
  search: string,
  hash: string,
): TelegramLoginPayload | null {
  return telegramPayloadFromSearch(search) ?? telegramPayloadFromHash(hash);
}

export interface TelegramOAuthError {
  code: string | null;
  description: string | null;
}

/** Telegram returns OAuth failures as query/hash params, not as JSON. */
export function telegramErrorFromLocation(
  search: string,
  hash: string,
): TelegramOAuthError | null {
  const query = new URLSearchParams(search);
  const fragment = new URLSearchParams(hash.startsWith("#") ? hash.slice(1) : hash);
  const code =
    query.get("error") ??
    query.get("error_code") ??
    fragment.get("error") ??
    fragment.get("error_code");
  const description =
    query.get("error_description") ??
    fragment.get("error_description");
  if (!code && !description) {
    return null;
  }
  return { code, description };
}

export function telegramErrorMentionsDomain(
  error: TelegramOAuthError,
): boolean {
  return /domain|origin|bot/i.test(`${error.code ?? ""} ${error.description ?? ""}`);
}

function payloadFromMessageValue(
  value: unknown,
  depth: number,
): TelegramLoginPayload | null {
  if (depth > 4) {
    return null;
  }
  if (typeof value === "string") {
    const direct = telegramPayloadFromAuthResult(value);
    if (direct) {
      return direct;
    }
    const hashPayload = telegramPayloadFromHash(value);
    if (hashPayload) {
      return hashPayload;
    }
    const searchPayload = telegramPayloadFromSearch(
      value.startsWith("?") ? value : `?${value}`,
    );
    if (searchPayload) {
      return searchPayload;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(value) as unknown;
    } catch {
      return null;
    }
    return payloadFromMessageValue(parsed, depth + 1);
  }
  if (isJsonObject(value)) {
    const direct = payloadFromRecord(value);
    if (direct) {
      return direct;
    }
    for (const key of ["tgAuthResult", "result", "data", "payload"]) {
      const nested = value[key];
      if (typeof nested === "string" || isJsonObject(nested)) {
        const payload = payloadFromMessageValue(nested, depth + 1);
        if (payload) {
          return payload;
        }
      }
    }
  }
  return null;
}

export function telegramPayloadFromMessage(
  data: unknown,
): TelegramLoginPayload | null {
  return payloadFromMessageValue(data, 0);
}
