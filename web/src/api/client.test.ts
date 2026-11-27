import { describe, expect, it } from "vitest";

import type { AuthSessionRecord } from "../db/storage";
import {
  ApiClient,
  inviteCodeFromUrl,
  type SyncRequest,
} from "./client";

interface FetchCall {
  url: string;
  method: string;
  authorization: string | null;
  body: string | null;
}

function requestUrl(input: RequestInfo | URL): string {
  if (typeof input === "string") {
    return input;
  }
  if (input instanceof URL) {
    return input.toString();
  }
  return input.url;
}

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("API client", () => {
  it("captures an invite from the Telegram return URL", async () => {
    const calls: FetchCall[] = [];
    const auth: { session: AuthSessionRecord | null } = { session: null };
    const client = new ApiClient({
      now: () => 1_000,
      fetchImpl: async (input, init) => {
        const headers = new Headers(init?.headers);
        calls.push({
          url: requestUrl(input),
          method: init?.method ?? "GET",
          authorization: headers.get("authorization"),
          body: typeof init?.body === "string" ? init.body : null,
        });
        return jsonResponse({
          access_token: "access",
          refresh_token: "refresh",
          learner_id: "learner",
          expires_in: 900,
        });
      },
      getSession: async () => auth.session,
      setSession: async (next) => {
        auth.session = next;
      },
    });

    const result = await client.telegramLogin(
      {
        id: 42,
        first_name: "Test",
        auth_date: 1,
        hash: "hash",
      },
      "?invite=invite-123",
    );

    expect("access_token" in result).toBe(true);
    expect(calls).toHaveLength(1);
    const body: unknown = JSON.parse(calls[0]?.body ?? "{}");
    expect(body).toMatchObject({ invite_code: "invite-123" });
    expect(auth.session?.access_token).toBe("access");
    expect(inviteCodeFromUrl("?invite=abc")).toBe("abc");
    expect(inviteCodeFromUrl("?other=1")).toBeNull();
  });

  it("refreshes an expired access token once and retries the request", async () => {
    const calls: FetchCall[] = [];
    let session: AuthSessionRecord | null = {
      learner_id: "learner",
      access_token: "old-access",
      refresh_token: "refresh",
      expires_at: 1_000_000,
    };
    const client = new ApiClient({
      now: () => 1_000,
      fetchImpl: async (input, init) => {
        const headers = new Headers(init?.headers);
        const url = requestUrl(input);
        calls.push({
          url,
          method: init?.method ?? "GET",
          authorization: headers.get("authorization"),
          body: typeof init?.body === "string" ? init.body : null,
        });
        if (url === "/v1/auth/refresh") {
          return jsonResponse({
            access_token: "new-access",
            refresh_token: "new-refresh",
            learner_id: "learner",
            expires_in: 900,
          });
        }
        if (calls.filter((call) => call.url === "/v1/sync").length === 1) {
          return jsonResponse({ error: "unauthorized", message: "expired" }, 401);
        }
        return jsonResponse({
          accepted: [],
          rejected: [],
          events: [],
          cursor: 0,
          more: false,
          server_time: 1_000,
          min_core_version: "0.0.0",
          latest_content_version: null,
        });
      },
      getSession: async () => session,
      setSession: async (next) => {
        session = next;
      },
    });
    const request: SyncRequest = {
      learner_id: "learner",
      core_version: "0.0.0",
      content_version: "content-1",
      cursor: 0,
      events: [],
    };

    await client.sync(request);

    expect(calls.map((call) => call.url)).toEqual([
      "/v1/sync",
      "/v1/auth/refresh",
      "/v1/sync",
    ]);
    expect(calls[0]?.authorization).toBe("Bearer old-access");
    expect(calls[2]?.authorization).toBe("Bearer new-access");
    expect(session?.access_token).toBe("new-access");
  });
});
