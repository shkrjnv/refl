// @vitest-environment jsdom

import { readFileSync } from "node:fs";

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

function cssRule(css: string, selector: string): string {
  const marker = css.indexOf("/* W17");
  const start = css.indexOf(selector, marker >= 0 ? marker : 0);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  return css.slice(open + 1, close);
}

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

  it("renders the editorial studio copy and all five palette chips", () => {
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
    for (const name of [
      "Deep Umber",
      "Burgundy Studio",
      "Midnight Cobalt",
      "Cream Paper",
      "Sand & Forest",
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }

    const choices = Array.from(
      document.querySelectorAll<HTMLButtonElement>(
        ".auth-panel-topbar .palette-choice",
      ),
    );
    expect(choices).toHaveLength(5);
    expect(
      document.querySelectorAll(".auth-panel-topbar .appearance-chip-swatch"),
    ).toHaveLength(5);
    expect(choices.filter((choice) => choice.classList.contains("active"))).toHaveLength(
      1,
    );
    expect(
      choices
        .find((choice) => choice.classList.contains("active"))
        ?.querySelector(".palette-swatch-burgundy"),
    ).not.toBeNull();

    const css = readFileSync("src/app.css", "utf8");
    const toggle = cssRule(css, ".auth-panel-topbar .palette-toggle {");
    expect(toggle).toContain("gap: 6px");
    expect(toggle).toContain("padding: 4px");
    const choice = cssRule(
      css,
      ".auth-panel-topbar .palette-choice,\n.auth-panel-topbar .palette-choice.active {",
    );
    expect(choice).toContain("width: 28px");
    expect(choice).toContain("min-height: 28px");
    const activeSwatch = cssRule(
      css,
      ".auth-panel-topbar .palette-choice.active .appearance-chip-swatch {",
    );
    expect(activeSwatch).toContain("outline: 2px solid var(--text)");
    expect(activeSwatch).toContain("outline-offset: 2px");
    expect(activeSwatch).not.toContain("0 0 0 3.5px");
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
      css.lastIndexOf("@media (prefers-reduced-motion: reduce)"),
    );
    expect(reducedMotion).toContain(".auth-form-code");
    expect(reducedMotion).toContain(".auth-form-region .ui-button");
    expect(reducedMotion).toContain(".studio-main");
  });

  it("enters the app directly without a duplicate success overlay", async () => {
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
    await waitFor(() => expect(authenticated).not.toBeNull(), { timeout: 2000 });
    expect(screen.queryByText("Signed in")).toBeNull();
    expect(document.querySelector(".auth-success")).toBeNull();
  });
});
