import { describe, expect, it } from "vitest";

import enSource from "./locales/en.ftl?raw";
import ruSource from "./locales/ru.ftl?raw";
import uzSource from "./locales/uz.ftl?raw";
import { createBundle, formatMessage, resolveLocale } from "./index";

describe("Fluent locales", () => {
  it("parses en, ru, and uz resources", () => {
    expect(() => createBundle("en", enSource)).not.toThrow();
    expect(() => createBundle("ru", ruSource)).not.toThrow();
    expect(() => createBundle("uz", uzSource)).not.toThrow();
  });

  it("formats localized messages and falls back to English", () => {
    const en = createBundle("en", enSource);
    const ru = createBundle("ru", ruSource);
    const uz = createBundle("uz", uzSource);
    expect(formatMessage(ru, en, "auth-code-sent", { minutes: 10 })).toContain(
      "10",
    );
    expect(formatMessage(uz, en, "auth-code-sent", { minutes: 10 })).toContain(
      "10",
    );
    expect(formatMessage(en, en, "missing-message")).toBe("missing-message");
    expect(formatMessage(ru, en, "app-name")).toBe("refl");
  });

  it("prefers a persisted locale and negotiates a regional browser locale", () => {
    expect(resolveLocale(["en-US"], "ru")).toBe("ru");
    expect(resolveLocale(["ru-RU"], null)).toBe("ru");
    expect(resolveLocale(["uz-UZ"], null)).toBe("uz");
    expect(resolveLocale(["fr-FR"], null)).toBe("en");
  });
});
