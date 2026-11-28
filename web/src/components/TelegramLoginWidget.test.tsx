// @vitest-environment jsdom

import { cleanup, render, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { TelegramLoginPayload } from "../api/client";
import { TelegramLoginWidget } from "./TelegramLoginWidget";

afterEach(cleanup);

describe("Telegram Login Widget", () => {
  it("loads the widget script for the configured bot and forwards the hash payload", async () => {
    let received: TelegramLoginPayload | null = null;
    render(
      <TelegramLoginWidget
        username="refl_bot"
        onAuth={(payload) => {
          received = payload;
        }}
        onError={() => undefined}
      />,
    );

    const script = document.querySelector<HTMLScriptElement>(
      'script[data-telegram-login="refl_bot"]',
    );
    expect(script).toBeTruthy();
    expect(script?.src).toBe("https://telegram.org/js/telegram-widget.js?22");
    const onauth = script?.getAttribute("data-onauth") ?? "";
    const callbackName = onauth.replace("(user)", "");
    const payload = {
      id: 4242,
      first_name: "Aziza",
      last_name: null,
      username: "aziza",
      photo_url: null,
      auth_date: 1_700_000_000,
      hash: "server-hash",
    };
    const callback = (
      window as unknown as Record<
        string,
        ((user: unknown) => void) | undefined
      >
    )[callbackName];
    expect(typeof callback).toBe("function");
    callback?.(payload);
    await waitFor(() => expect(received).toEqual(payload));
  });

  it("reports an invalid username instead of injecting an unusable widget", async () => {
    let failed = false;
    render(
      <TelegramLoginWidget
        username="not a bot"
        onAuth={() => undefined}
        onError={() => {
          failed = true;
        }}
      />,
    );
    await waitFor(() => expect(failed).toBe(true));
    expect(document.querySelector("script[data-telegram-login]")).toBeNull();
  });
});
