import { describe, expect, it } from "vitest";

import type { TelegramLoginPayload } from "../api/client";
import {
  telegramErrorFromLocation,
  telegramErrorMentionsDomain,
  telegramPayloadFromAuthResult,
  telegramPayloadFromHash,
  telegramPayloadFromLocation,
  telegramPayloadFromMessage,
  telegramPayloadFromSearch,
  telegramPayloadFromWebApp,
} from "./telegram";

const PAYLOAD: TelegramLoginPayload = {
  id: 4242,
  first_name: "Aziza",
  last_name: "Karimova",
  username: "aziza",
  photo_url: null,
  auth_date: 1_700_000_000,
  hash: "server-hash",
};

function base64(value: TelegramLoginPayload): string {
  return Buffer.from(JSON.stringify(value), "utf8").toString("base64");
}

describe("Telegram OAuth return parsing", () => {
  it("parses the base64 #tgAuthResult payload", () => {
    expect(telegramPayloadFromHash(`#tgAuthResult=${base64(PAYLOAD)}`)).toEqual(
      PAYLOAD,
    );
  });

  it("accepts a percent-encoded hash value and ignores following fields", () => {
    const encoded = encodeURIComponent(base64(PAYLOAD));
    expect(
      telegramPayloadFromHash(`#other=1&tgAuthResult=${encoded}&next=2`),
    ).toEqual(PAYLOAD);
  });

  it("still accepts a classic query-string login return", () => {
    const search =
      "?id=4242&first_name=Aziza&last_name=Karimova&username=aziza" +
      "&auth_date=1700000000&hash=server-hash";
    expect(telegramPayloadFromSearch(search)).toEqual(PAYLOAD);
    expect(telegramPayloadFromLocation(search, "")).toEqual(PAYLOAD);
  });

  it("parses postMessage wrappers without trusting their shape", () => {
    const encoded = base64(PAYLOAD);
    expect(telegramPayloadFromMessage({ tgAuthResult: encoded })).toEqual(
      PAYLOAD,
    );
    expect(
      telegramPayloadFromMessage({ event: "auth_result", result: PAYLOAD }),
    ).toEqual(PAYLOAD);
    expect(
      telegramPayloadFromMessage(JSON.stringify({ payload: PAYLOAD })),
    ).toEqual(PAYLOAD);
  });

  it("rejects malformed results instead of returning a partial payload", () => {
    expect(telegramPayloadFromAuthResult("not-base64-json")).toBeNull();
    expect(telegramPayloadFromHash("#tgAuthResult=")).toBeNull();
    expect(telegramPayloadFromHash("#other=value")).toBeNull();
    expect(
      telegramPayloadFromMessage({ tgAuthResult: base64({ ...PAYLOAD, first_name: "" }) }),
    ).toBeNull();
    expect(telegramPayloadFromMessage({ result: { id: 4242 } })).toBeNull();
  });

  it("keeps raw Mini App initData for server-side verification", () => {
    const user = encodeURIComponent(
      JSON.stringify({ id: 4242, first_name: "Aziza" }),
    );
    const initData = `auth_date=1700000000&query_id=AAH-test&user=${user}&hash=abc`;
    const payload = telegramPayloadFromWebApp(initData);
    expect(payload?.init_data).toBe(initData);
    expect(payload?.id).toBe(4242);
  });

  it("surfaces Telegram OAuth and domain errors instead of dropping them", () => {
    expect(
      telegramErrorFromLocation("?error=access_denied", ""),
    ).toEqual({ code: "access_denied", description: null });
    const domain = telegramErrorFromLocation(
      "?error=bot_domain_invalid&error_description=Invalid+domain",
      "",
    );
    expect(domain).toEqual({
      code: "bot_domain_invalid",
      description: "Invalid domain",
    });
    expect(domain && telegramErrorMentionsDomain(domain)).toBe(true);
    expect(
      telegramErrorMentionsDomain({ code: "access_denied", description: null }),
    ).toBe(false);
    expect(telegramErrorFromLocation("", "")).toBeNull();
  });
});
