// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { AuthTokens, VerifyCodeResponse } from "../api/client";
import { I18nProvider } from "../i18n";
import type { AuthApi } from "./AuthScreen";
import { AuthGate } from "./AuthGate";

afterEach(() => {
  cleanup();
  window.history.replaceState({}, "", "/");
});

describe("auth and access flow", () => {
  it("keeps the invite and moves to pending approval", async () => {
    window.history.replaceState(
      {},
      "",
      "/?invite=INVITE-42",
    );
    let authenticated: AuthTokens | null = null;
    let verifiedInvite: string | null = null;
    const api: AuthApi = {
      requestCode: async () => ({
        sent: true,
        expires_in: 600,
        code: "123456",
      }),
      verifyCode: async (
        _email,
        _code,
        invite,
      ): Promise<VerifyCodeResponse> => {
        verifiedInvite = invite ?? null;
        return {
          status: "pending_approval",
          learner_id: "learner-42",
        };
      },
      telegramLogin: async () => ({
        access_token: "a",
        refresh_token: "r",
        learner_id: "learner-42",
        expires_in: 900,
      }),
    };

    render(
      <I18nProvider initialLocale="en">
        <AuthGate
          api={api}
          inviteCode="INVITE-42"
          onAuthenticated={(tokens) => {
            authenticated = tokens;
          }}
        />
      </I18nProvider>,
    );

    fireEvent.change(screen.getByLabelText("Email"), {
      target: { value: "learner@example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send code" }));
    await screen.findByText("Development code: 123456");

    fireEvent.change(screen.getByLabelText("Code from the email"), {
      target: { value: "123456" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    await screen.findByText("Waiting for approval");
    await waitFor(() => expect(verifiedInvite).toBe("INVITE-42"));
    expect(authenticated).toBeNull();
  });

  it("consumes a Telegram #tgAuthResult hash return once", async () => {
    const payload = {
      id: 4242,
      first_name: "Aziza",
      last_name: null,
      username: "aziza",
      photo_url: null,
      auth_date: 1_700_000_000,
      hash: "server-hash",
    };
    const encoded = Buffer.from(JSON.stringify(payload), "utf8").toString(
      "base64",
    );
    window.history.replaceState(
      {},
      "",
      `/?invite=INVITE-42#tgAuthResult=${encoded}`,
    );
    let authenticated: AuthTokens | null = null;
    let received: unknown = null;
    const api: AuthApi = {
      requestCode: async () => ({ sent: true, expires_in: 600 }),
      verifyCode: async () => ({
        access_token: "a",
        refresh_token: "r",
        learner_id: "learner-42",
        expires_in: 900,
      }),
      telegramLogin: async (loginPayload) => {
        received = loginPayload;
        return {
          access_token: "a",
          refresh_token: "r",
          learner_id: "learner-42",
          expires_in: 900,
        };
      },
    };

    render(
      <I18nProvider initialLocale="en">
        <AuthGate
          api={api}
          inviteCode="INVITE-42"
          onAuthenticated={(tokens) => {
            authenticated = tokens;
          }}
        />
      </I18nProvider>,
    );

    await waitFor(() => expect(received).toEqual(payload));
    await waitFor(() => expect(authenticated).not.toBeNull());
    expect(window.location.hash).toBe("");
    expect(window.location.search).toBe("?invite=INVITE-42");
  });
});
