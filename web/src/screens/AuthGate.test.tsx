// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";

import type { AuthTokens, VerifyCodeResponse } from "../api/client";
import { I18nProvider } from "../i18n";
import { applyTheme, storeTheme, type ThemeName } from "../theme/tokens";
import type { AuthApi } from "./AuthScreen";
import { AuthGate } from "./AuthGate";

afterEach(() => {
  cleanup();
  window.localStorage.clear();
  window.history.replaceState({}, "", "/");
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("data-palette");
  document.documentElement.style.removeProperty("color-scheme");
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
          theme="deep-umber"
          skin="living-studio"
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
          onThemeChange={() => undefined}
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
          theme="deep-umber"
          skin="living-studio"
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
          onThemeChange={() => undefined}
          onAuthenticated={(tokens) => {
            authenticated = tokens;
          }}
        />
      </I18nProvider>,
    );

    await waitFor(() => expect(received).toEqual(payload));
    await waitFor(() => expect(authenticated).not.toBeNull(), { timeout: 2000 });
    expect(window.location.hash).toBe("");
    expect(window.location.search).toBe("?invite=INVITE-42");
  });

  it("persists the palette chosen before sign-in", () => {
    let selected: ThemeName = "deep-umber";
    function Harness() {
      const [theme, setTheme] = useState<ThemeName>("deep-umber");
      return (
        <I18nProvider initialLocale="en">
          <AuthGate
            api={{
              requestCode: async () => ({ sent: true, expires_in: 600 }),
              verifyCode: async () => ({
                access_token: "a",
                refresh_token: "r",
                learner_id: "learner-42",
                expires_in: 900,
              }),
              telegramLogin: async () => ({
                access_token: "a",
                refresh_token: "r",
                learner_id: "learner-42",
                expires_in: 900,
              }),
            }}
            inviteCode={null}
            theme={theme}
            skin="living-studio"
            locale="en"
            locales={["en", "ru", "uz"]}
            onLocaleChange={() => undefined}
            onThemeChange={(next) => {
              selected = next;
              storeTheme(next);
              applyTheme(next);
              setTheme(next);
            }}
            onAuthenticated={() => undefined}
          />
        </I18nProvider>
      );
    }
    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "Cream Paper" }));
    expect(selected).toBe("cream-paper");
    expect(window.localStorage.getItem("refl.theme")).toBe("cream-paper");
    expect(window.localStorage.getItem("refl.palette")).toBe("cream-paper");
    expect(document.documentElement.dataset["palette"]).toBe("cream-paper");
    expect(document.documentElement.dataset["theme"]).toBe("light");
  });

  it("shows the confirmation check before entering the app", async () => {
    let authenticated: AuthTokens | null = null;
    const api: AuthApi = {
      requestCode: async () => ({ sent: true, expires_in: 600, code: "123456" }),
      verifyCode: async () => ({
        access_token: "a",
        refresh_token: "r",
        learner_id: "learner-42",
        expires_in: 900,
      }),
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
          inviteCode={null}
          theme="deep-umber"
          skin="living-studio"
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
          onThemeChange={() => undefined}
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
    expect(screen.getByText("Enter the code")).toBeTruthy();
    expect(screen.getByText(/l\*\*\*\*\*\*@example\.com/)).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Code from the email"), {
      target: { value: "123456" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    await screen.findByText("Signed in");
    expect(authenticated).toBeNull();
    await waitFor(() => expect(authenticated).not.toBeNull(), { timeout: 2000 });
  });
});
