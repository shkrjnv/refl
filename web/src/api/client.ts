import type { AuthSessionRecord, LearnerStorage } from "../db/storage";
import { isJsonObject, type JsonValue } from "../types/json";

export interface RequestCodeResponse {
  sent: boolean;
  expires_in: number;
  code?: string;
  dev_code?: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  learner_id: string;
  expires_in: number;
}

export interface PendingApproval {
  status: "pending_approval";
  learner_id: string;
}

export type VerifyCodeResponse = AuthTokens | PendingApproval;

export interface TelegramLoginPayload {
  id: number;
  first_name: string;
  last_name?: string | null;
  username?: string | null;
  photo_url?: string | null;
  auth_date: number;
  hash: string;
  invite_code?: string | null;
}

export interface CreatedInvite {
  id: string;
  invite_code: string;
  max_uses: number;
  expires_at: number | null;
}

export interface LoginMethodView {
  kind: string;
  identifier: string;
  verified_at: number | null;
}

export interface PendingUser {
  user_id: string;
  display_name: string;
  created_at: number;
  login_methods: LoginMethodView[];
}

export interface DecisionResponse {
  user_id: string;
  access_status: string;
}

export interface RejectedEvent {
  event_id: string;
  reason: string;
}

export interface SyncRequest {
  learner_id: string;
  core_version: string;
  content_version: string;
  cursor: number;
  events: JsonValue[];
}

export interface SyncResponse {
  accepted: string[];
  rejected: RejectedEvent[];
  events: JsonValue[];
  cursor: number;
  more: boolean;
  server_time: number;
  min_core_version: string;
  latest_content_version: string | null;
}

export interface ErasedResponse {
  erased: true;
  status: "erased";
}

export interface ActiveContentResponse {
  content_version: string;
  min_core_version: string;
  digest: string;
  bundle: JsonValue;
}

export type SyncResultResponse = SyncResponse | ErasedResponse;

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly minCoreVersion?: string;

  constructor(
    status: number,
    code: string,
    message: string,
    minCoreVersion?: string,
  ) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.minCoreVersion = minCoreVersion;
  }
}

export interface ApiClientOptions {
  baseUrl?: string;
  fetchImpl?: typeof globalThis.fetch;
  getSession?: () => Promise<AuthSessionRecord | null>;
  setSession?: (session: AuthSessionRecord | null) => Promise<void>;
  now?: () => number;
}

type FetchBody = string | undefined;

function currentSearch(): string {
  return typeof window === "undefined" ? "" : window.location.search;
}

export function inviteCodeFromUrl(search: string = currentSearch()): string | null {
  const code = new URLSearchParams(search).get("invite")?.trim();
  return code ? code : null;
}

export function isAuthTokens(value: VerifyCodeResponse): value is AuthTokens {
  return "access_token" in value;
}

function isErasedResponse(value: unknown): value is ErasedResponse {
  return isJsonObject(value) && value["erased"] === true;
}

function errorFromBody(
  status: number,
  body: unknown,
): ApiError {
  if (isJsonObject(body)) {
    const code = body["error"];
    const message = body["message"];
    const min = body["min_core_version"];
    return new ApiError(
      status,
      typeof code === "string" ? code : "http_error",
      typeof message === "string" ? message : `HTTP ${status}`,
      typeof min === "string" ? min : undefined,
    );
  }
  return new ApiError(status, "http_error", `HTTP ${status}`);
}

export function createStoredApiClient(
  storage: LearnerStorage,
  options: Omit<ApiClientOptions, "getSession" | "setSession"> = {},
): ApiClient {
  return new ApiClient({
    ...options,
    getSession: () => storage.getAuthSession(),
    setSession: (session) => storage.setAuthSession(session),
  });
}

export class ApiClient {
  readonly #baseUrl: string;
  readonly #fetch: typeof globalThis.fetch;
  readonly #getSession: () => Promise<AuthSessionRecord | null>;
  readonly #setSession: (session: AuthSessionRecord | null) => Promise<void>;
  readonly #now: () => number;

  constructor(options: ApiClientOptions = {}) {
    this.#baseUrl = (options.baseUrl ?? "").replace(/\/$/, "");
    this.#fetch = options.fetchImpl ?? globalThis.fetch.bind(globalThis);
    this.#getSession = options.getSession ?? (async () => null);
    this.#setSession = options.setSession ?? (async () => undefined);
    this.#now = options.now ?? Date.now;
  }

  async requestCode(email: string): Promise<RequestCodeResponse> {
    return this.#requestJson<RequestCodeResponse>(
      "/v1/auth/request-code",
      { method: "POST", body: JSON.stringify({ email }) },
      false,
    );
  }

  async verifyCode(
    email: string,
    code: string,
    inviteCode: string | null = inviteCodeFromUrl(),
  ): Promise<VerifyCodeResponse> {
    const body: Record<string, string> = { email, code };
    if (inviteCode) {
      body.invite_code = inviteCode;
    }
    const response = await this.#requestJson<VerifyCodeResponse>(
      "/v1/auth/verify-code",
      { method: "POST", body: JSON.stringify(body) },
      false,
    );
    if (isAuthTokens(response)) {
      await this.#adoptTokens(response);
    }
    return response;
  }

  async telegramLogin(
    payload: TelegramLoginPayload,
    search: string = currentSearch(),
  ): Promise<VerifyCodeResponse> {
    const invite = payload.invite_code ?? inviteCodeFromUrl(search);
    const body = invite ? { ...payload, invite_code: invite } : payload;
    const response = await this.#requestJson<VerifyCodeResponse>(
      "/v1/auth/telegram",
      { method: "POST", body: JSON.stringify(body) },
      false,
    );
    if (isAuthTokens(response)) {
      await this.#adoptTokens(response);
    }
    return response;
  }

  async refreshSession(): Promise<AuthSessionRecord> {
    const session = await this.#getSession();
    if (!session || session.refresh_token.length === 0) {
      throw new ApiError(401, "unauthorized", "no refresh token");
    }
    try {
      const tokens = await this.#requestJson<AuthTokens>(
        "/v1/auth/refresh",
        {
          method: "POST",
          body: JSON.stringify({ refresh_token: session.refresh_token }),
        },
        false,
        false,
      );
      return await this.#adoptTokens(tokens);
    } catch (error) {
      if (
        error instanceof ApiError &&
        (error.status === 401 || error.status === 403)
      ) {
        await this.#setSession(null);
      }
      throw error;
    }
  }

  async logout(): Promise<{ logged_out: boolean }> {
    const session = await this.#getSession();
    if (!session) {
      return { logged_out: true };
    }
    const result = await this.#requestJson<{ logged_out: boolean }>(
      "/v1/auth/logout",
      {
        method: "POST",
        body: JSON.stringify({ refresh_token: session.refresh_token }),
      },
      false,
    );
    await this.#setSession(null);
    return result;
  }

  async createInvite(input: {
    note?: string;
    max_uses: number;
    expires_in_hours?: number;
  }): Promise<CreatedInvite> {
    return this.#requestJson<CreatedInvite>(
      "/v1/admin/invites",
      { method: "POST", body: JSON.stringify(input) },
      true,
    );
  }

  async pendingRequests(): Promise<PendingUser[]> {
    return this.#requestJson<PendingUser[]>(
      "/v1/admin/requests",
      { method: "GET" },
      true,
    );
  }

  async approve(userId: string): Promise<DecisionResponse> {
    return this.#requestJson<DecisionResponse>(
      `/v1/admin/requests/${encodeURIComponent(userId)}/approve`,
      { method: "POST" },
      true,
    );
  }

  async reject(userId: string): Promise<DecisionResponse> {
    return this.#requestJson<DecisionResponse>(
      `/v1/admin/requests/${encodeURIComponent(userId)}/reject`,
      { method: "POST" },
      true,
    );
  }

  async activeContent(): Promise<ActiveContentResponse> {
    return this.#requestJson<ActiveContentResponse>(
      "/v1/content/active",
      { method: "GET" },
      false,
    );
  }

  async sync(request: SyncRequest): Promise<SyncResultResponse> {
    return this.#requestJson<SyncResultResponse>(
      "/v1/sync",
      { method: "POST", body: JSON.stringify(request) },
      true,
    );
  }

  async #adoptTokens(tokens: AuthTokens): Promise<AuthSessionRecord> {
    const session: AuthSessionRecord = {
      learner_id: tokens.learner_id,
      access_token: tokens.access_token,
      refresh_token: tokens.refresh_token,
      expires_at: this.#now() + tokens.expires_in * 1000,
    };
    await this.#setSession(session);
    return session;
  }

  async #requestJson<T>(
    path: string,
    init: { method: string; body?: FetchBody },
    authenticated: boolean,
    allowRefresh = true,
  ): Promise<T> {
    let session = authenticated ? await this.#getSession() : null;
    if (
      authenticated &&
      session &&
      session.expires_at - this.#now() <= 30_000 &&
      allowRefresh
    ) {
      session = await this.refreshSession();
    }
    const headers = new Headers();
    if (init.body !== undefined) {
      headers.set("content-type", "application/json");
    }
    if (session) {
      headers.set("authorization", `Bearer ${session.access_token}`);
    }
    const response = await this.#fetch(`${this.#baseUrl}${path}`, {
      method: init.method,
      headers,
      body: init.body,
    });
    const text = await response.text();
    let body: unknown = null;
    if (text.length > 0) {
      body = JSON.parse(text) as unknown;
    }
    if (!response.ok) {
      if (response.status === 401 && authenticated && allowRefresh) {
        await this.refreshSession();
        return this.#requestJson<T>(path, init, authenticated, false);
      }
      throw errorFromBody(response.status, body);
    }
    return body as T;
  }
}

export { isErasedResponse };
