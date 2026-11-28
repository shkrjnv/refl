// @vitest-environment jsdom

import "fake-indexeddb/auto";

import { readFileSync } from "node:fs";

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it } from "vitest";

import type { AuthTokens, VerifyCodeResponse } from "../api/client";
import { deleteLearnerStorage, LearnerStorage } from "../db/storage";
import { I18nProvider } from "../i18n";
import { storeSessionOwner } from "../sync/session";
import { applyTheme, storeTheme, type ThemeName } from "../theme/tokens";
import { confirmationHoldMs, type AuthApi } from "./AuthScreen";
import { AuthGate } from "./AuthGate";

afterEach(() => {
  cleanup();
  window.localStorage.clear();
  window.history.replaceState({}, "", "/");
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("data-palette");
  document.documentElement.style.removeProperty("color-scheme");
});

function cssRule(css: string, selector: string): string {
  const marker = css.indexOf("/* W17");
  const start = css.indexOf(selector, marker >= 0 ? marker : 0);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  return css.slice(open + 1, close);
}

describe("auth and access flow", () => {
  it("holds the confirmation check long enough, shorter under reduced motion", () => {
    expect(confirmationHoldMs()).toBe(650);
    const original = window.matchMedia;
    window.matchMedia = ((query: string): MediaQueryList => ({
      matches: true,
      media: query,
      onchange: null,
      addListener: () => undefined,
      removeListener: () => undefined,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      dispatchEvent: () => false,
    })) as typeof window.matchMedia;
    try {
      expect(confirmationHoldMs()).toBe(250);
    } finally {
      window.matchMedia = original;
    }
  });

  it("restores a persisted session across AuthGate remounts", async () => {
    const learnerId = "auth-restore-learner";
    const store = await LearnerStorage.open(learnerId);
    await store.setAuthSession({
      learner_id: learnerId,
      access_token: "stored-access",
      refresh_token: "stored-refresh",
      expires_at: Date.now() + 60_000,
    });
    store.close();
    storeSessionOwner({ learnerId, method: "telegram" });

    const received: Array<{ tokens: AuthTokens; method: string }> = [];
    const api: AuthApi = {
      requestCode: async () => ({ sent: true, expires_in: 600 }),
      verifyCode: async () => ({
        access_token: "a",
        refresh_token: "r",
        learner_id: learnerId,
        expires_in: 900,
      }),
      telegramLogin: async () => ({
        access_token: "a",
        refresh_token: "r",
        learner_id: learnerId,
        expires_in: 900,
      }),
    };
    const gate = () => (
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
          onAuthenticated={(tokens, method) => {
            received.push({ tokens, method });
          }}
        />
      </I18nProvider>
    );

    const first = render(gate());
    await waitFor(() => expect(received).toHaveLength(1));
    expect(received[0]?.tokens.access_token).toBe("stored-access");
    expect(received[0]?.tokens.refresh_token).toBe("stored-refresh");
    expect(received[0]?.tokens.learner_id).toBe(learnerId);
    expect(received[0]?.method).toBe("telegram");
    expect(screen.queryByLabelText("Email")).toBeNull();
    first.unmount();

    const second = render(gate());
    await waitFor(() => expect(received).toHaveLength(2));
    expect(received[1]?.tokens.access_token).toBe("stored-access");
    expect(screen.queryByLabelText("Email")).toBeNull();
    second.unmount();

    await deleteLearnerStorage(learnerId);
  });

  it("persists a completed sign-in and restores it after remounting", async () => {
    const learnerId = "auth-login-persist";
    const received: Array<{ tokens: AuthTokens; method: string }> = [];
    const api: AuthApi = {
      requestCode: async () => ({
        sent: true,
        expires_in: 600,
        code: "123456",
      }),
      verifyCode: async () => ({
        access_token: "login-access",
        refresh_token: "login-refresh",
        learner_id: learnerId,
        expires_in: 900,
      }),
      telegramLogin: async () => ({
        access_token: "login-access",
        refresh_token: "login-refresh",
        learner_id: learnerId,
        expires_in: 900,
      }),
    };
    const gate = () => (
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
          onAuthenticated={(tokens, method) => {
            received.push({ tokens, method });
          }}
        />
      </I18nProvider>
    );

    const first = render(gate());
    fireEvent.change(screen.getByLabelText("Email"), {
      target: { value: "learner@example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send code" }));
    await screen.findByText("Development code: 123456");
    fireEvent.change(screen.getByLabelText("Code from the email"), {
      target: { value: "123456" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    await waitFor(() => expect(received).toHaveLength(1), { timeout: 2000 });
    expect(window.localStorage.getItem("refl.session-owner")).not.toBeNull();
    first.unmount();

    const second = render(gate());
    await waitFor(() => expect(received).toHaveLength(2), { timeout: 2000 });
    expect(received[1]?.tokens.access_token).toBe("login-access");
    expect(received[1]?.method).toBe("email");
    expect(screen.queryByLabelText("Email")).toBeNull();
    second.unmount();

    await deleteLearnerStorage(learnerId);
  });

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
    expect(await screen.findByText("Signed in")).toBeTruthy();
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
    fireEvent.click(
      screen.getByRole("button", { name: "Switch to light theme" }),
    );
    expect(selected).toBe("cream-paper");
    expect(window.localStorage.getItem("refl.theme")).toBe("cream-paper");
    expect(window.localStorage.getItem("refl.palette")).toBe("cream-paper");
    expect(document.documentElement.dataset["palette"]).toBe("cream-paper");
    expect(document.documentElement.dataset["theme"]).toBe("light");
  });

  it("renders the editorial studio copy and one scheme toggle", () => {
    render(
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
          theme="burgundy"
          skin="living-studio"
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
          onThemeChange={() => undefined}
          onAuthenticated={() => undefined}
        />
      </I18nProvider>,
    );

    expect(
      screen.getByRole("heading", { name: "Step into the studio" }),
    ).toBeTruthy();
    expect(
      screen.queryByText(
        "One-time code by email, or instant entry with Telegram.",
      ),
    ).toBeNull();
    const title = document.querySelector(".auth-form-header h2");
    expect(title?.textContent).toBe("Step into the studio");
    expect(title?.classList.contains("auth-title-liquid")).toBe(false);
    expect(title?.classList.contains("auth-form-title")).toBe(true);
    expect(document.querySelector(".auth-form-header .auth-card-subtitle")).toBeNull();
    expect(
      screen.getByRole("button", { name: "Switch to light theme" }),
    ).toBeTruthy();
    expect(document.querySelectorAll(".auth-panel-topbar .palette-choice"))
      .toHaveLength(0);
    expect(
      screen.queryByRole("button", { name: "Burgundy Studio" }),
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Sand & Forest" }),
    ).toBeNull();
  });

  it("renders one clean email input with a single inset focus ring", () => {
    render(
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
          theme="deep-umber"
          skin="living-studio"
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
          onThemeChange={() => undefined}
          onAuthenticated={() => undefined}
        />
      </I18nProvider>,
    );

    const inputs = Array.from(
      document.querySelectorAll<HTMLInputElement>(".auth-form input"),
    );
    expect(inputs).toHaveLength(1);
    expect(inputs[0]?.type).toBe("email");
    expect(inputs[0]?.getAttribute("style")).toBeNull();

    const css = readFileSync("src/app.css", "utf8");
    const emailFocus = cssRule(
      css,
      ".auth-form input:not(.auth-code-slot):focus,\n.auth-form input:not(.auth-code-slot):focus-visible {",
    );
    expect(emailFocus).toContain("outline: none");
    expect(emailFocus).toContain(
      "box-shadow: inset 0 0 0 1px var(--focus-ring)",
    );
    expect(emailFocus).not.toContain("outline-offset");
    expect(emailFocus).not.toContain("0 0 0 4px");

    const otpBase = cssRule(css, ".auth-code-slot {");
    expect(otpBase).toContain("box-shadow: none");
    const otpFocus = cssRule(
      css,
      ".auth-code-slot:focus,\n.auth-code-slot:focus-visible {",
    );
    expect(otpFocus).toContain("outline: none");
    expect(otpFocus).toContain(
      "box-shadow: inset 0 0 0 1px var(--focus-ring)",
    );
    expect(otpFocus).toContain("transform: none");
    expect(otpFocus).not.toContain("0 5px 0");

    const reducedMotion = css.slice(
      css.indexOf("@media (prefers-reduced-motion: reduce)", css.indexOf("/* W17")),
    );
    expect(reducedMotion).toContain(".auth-form-code");
    expect(reducedMotion).toContain(".auth-form-region .ui-button");
    expect(reducedMotion).toContain(".studio-main");
  });

  it("swaps to a clean confirmation state, then enters after the hold", async () => {
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
    expect(document.querySelector(".auth-confirm-check")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    const label = await screen.findByText("Signed in");
    const state = label.closest(".auth-confirm-state");
    expect(state).not.toBeNull();
    expect(state?.getAttribute("role")).toBe("status");
    expect(document.querySelector(".auth-form-title")).toBeNull();
    expect(document.querySelector(".auth-form-code")).toBeNull();
    expect(screen.queryByLabelText("Code from the email")).toBeNull();
    expect(document.querySelector(".auth-confirm-check")).toBeNull();
    const drawPath = state?.querySelector(".confirmation-check-path");
    expect(drawPath).not.toBeNull();
    expect(drawPath?.getAttribute("stroke-dasharray")).toBe("1");
    expect(drawPath?.getAttribute("stroke-dashoffset")).toBe("1");
    expect(document.querySelector(".pulse-confirm")).toBeNull();
    expect(authenticated).toBeNull();
    await waitFor(() => expect(authenticated).not.toBeNull(), { timeout: 2500 });
    expect(document.querySelector(".auth-success")).toBeNull();

    const css = readFileSync("src/app.css", "utf8");
    expect(css).not.toContain(".auth-confirm-check");
    const innerStart = css.indexOf(".auth-form-inner-confirmed {");
    const innerRule = css.slice(
      innerStart,
      css.indexOf("}", innerStart) + 1,
    );
    expect(innerRule).toContain("height: 100%");
    expect(innerRule).toContain("place-items: center");
    const ringStart = css.indexOf(".auth-confirm-mark {");
    const ringRule = css.slice(
      ringStart,
      css.indexOf("}", ringStart) + 1,
    );
    expect(ringRule).toContain("width: 68px");
    expect(ringRule).toContain("height: 68px");
    expect(ringRule).toContain("border: 2px solid var(--accent-primary)");
    expect(ringRule).toContain("border-radius: 50%");
    const drawStart = css.indexOf(
      ".auth-confirm-mark .confirmation-check-path {",
    );
    const drawRule = css.slice(
      drawStart,
      css.indexOf("}", drawStart) + 1,
    );
    expect(drawRule).toContain("stroke-dasharray: 1");
    expect(drawRule).toContain("stroke-dashoffset: 1");
    expect(drawRule).toContain(
      "animation: auth-confirm-draw 280ms cubic-bezier(0.22, 1, 0.36, 1)",
    );
    expect(css).toMatch(
      /prefers-reduced-motion: reduce[\s\S]*?\.auth-confirm-mark \.confirmation-check-path/,
    );
  });
});
