// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { AuthTokens, VerifyCodeResponse } from "../api/client";
import { I18nProvider } from "../i18n";
import type { AuthApi } from "./AuthScreen";
import { AuthGate } from "./AuthGate";

afterEach(cleanup);

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
});
