import type { ApiClient, AuthTokens } from "../api/client";
import {
  LearnerStorage,
  type AuthSessionRecord,
} from "../db/storage";

const SESSION_OWNER_KEY = "refl.session-owner";

export type SessionMethod = "email" | "telegram";

export interface SessionOwner {
  learnerId: string;
  method: SessionMethod;
}

export interface RestoredSession {
  tokens: AuthTokens;
  method: SessionMethod;
}

export function storeSessionOwner(owner: SessionOwner): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(SESSION_OWNER_KEY, JSON.stringify(owner));
  } catch {
    // Persistence can be unavailable in private or blocked contexts.
  }
}

export function readSessionOwner(): SessionOwner | null {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(SESSION_OWNER_KEY);
    if (!raw) {
      return null;
    }
    const value: unknown = JSON.parse(raw);
    if (typeof value !== "object" || value === null) {
      return null;
    }
    const learnerId = (value as Record<string, unknown>)["learnerId"];
    const method = (value as Record<string, unknown>)["method"];
    if (
      typeof learnerId !== "string" ||
      learnerId.trim().length === 0 ||
      (method !== "email" && method !== "telegram")
    ) {
      return null;
    }
    return { learnerId, method };
  } catch {
    return null;
  }
}

export function clearSessionOwner(): void {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.removeItem(SESSION_OWNER_KEY);
  } catch {
    // Persistence can be unavailable in private or blocked contexts.
  }
}

/**
 * Rebuild an AuthTokens payload from the last learner's stored session. An
 * expired access token is intentionally left expired: the first authenticated
 * request will refresh it with /v1/auth/refresh and keep the session.
 */
export async function restorePersistedSession(
  now: number = Date.now(),
): Promise<RestoredSession | null> {
  const owner = readSessionOwner();
  if (!owner) {
    return null;
  }
  let storage: LearnerStorage | null = null;
  try {
    storage = await LearnerStorage.open(owner.learnerId);
    const session = await storage.getAuthSession();
    if (!session || session.refresh_token.length === 0) {
      clearSessionOwner();
      return null;
    }
    return {
      method: owner.method,
      tokens: tokensFromSession(session, now),
    };
  } catch {
    return null;
  } finally {
    storage?.close();
  }
}

export function sessionRecordFromTokens(
  tokens: AuthTokens,
  now: number = Date.now(),
): AuthSessionRecord {
  return {
    learner_id: tokens.learner_id,
    access_token: tokens.access_token,
    refresh_token: tokens.refresh_token,
    expires_at: now + tokens.expires_in * 1000,
  };
}

export function tokensFromSession(
  session: AuthSessionRecord,
  now: number = Date.now(),
): AuthTokens {
  return {
    access_token: session.access_token,
    refresh_token: session.refresh_token,
    learner_id: session.learner_id,
    expires_in: Math.max(0, Math.floor((session.expires_at - now) / 1000)),
  };
}

/**
 * Sign out, then remove this learner's local database even when the network
 * logout request fails. Callers that still have unsynced events should sync
 * first; rejected events remain visible only until this wipe.
 */
export async function logoutAndWipe(
  api: Pick<ApiClient, "logout">,
  storage: LearnerStorage,
): Promise<void> {
  let failure: unknown = null;
  try {
    await api.logout();
  } catch (error) {
    failure = error;
  }
  try {
    await storage.wipe();
  } finally {
    clearSessionOwner();
  }
  if (failure !== null) {
    throw failure;
  }
}
