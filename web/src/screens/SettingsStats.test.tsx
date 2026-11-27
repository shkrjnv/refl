// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { initialStatus } from "../prototype/status";
import { SettingsScreen } from "./SettingsScreen";
import { StatsScreen } from "./StatsScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

const KC = "en.gram.article.a-an-sound";

describe("statistics and settings", () => {
  it("opens the source explanation behind a statistic", async () => {
    const content = testContent();
    const stateJson = JSON.stringify({
      kcs: {
        [KC]: {
          ever_mastered: true,
          observations: 4,
          card: { stability: 12, difficulty: 5, last_review: 1_700_000_000_000 },
        },
      },
    });

    render(
      <I18nProvider initialLocale="en">
        <StatsScreen content={content} stateJson={stateJson} />
      </I18nProvider>,
    );

    const buttons = screen.getAllByRole("button", {
      name: /Where does this come from\?/,
    });
    const first = buttons[0];
    if (!first) {
      throw new Error("missing disclosure");
    }
    fireEvent.click(first);
    expect(
      await screen.findByText(/mastery criterion has held/),
    ).toBeTruthy();
    expect(screen.getByText("Mastered KCs")).toBeTruthy();
  });

  it("switches language, skin, and mode from settings", () => {
    const content = testContent();
    let locale = "en";
    let selectedTheme = "light";
    let selectedSkin = "living-studio";
    render(
      <I18nProvider initialLocale="en">
        <SettingsScreen
          learnerLabel="Learner"
          authMethod="email"
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={(next) => {
            locale = next;
          }}
          theme="light"
          onThemeChange={(next) => {
            selectedTheme = next;
          }}
          skin="living-studio"
          onSkinChange={(next) => {
            selectedSkin = next;
          }}
          onReplayTour={() => undefined}
          status={initialStatus(true)}
          rejected={[]}
          onLogout={() => undefined}
          isAdmin={false}
          adminApi={null}
        />
      </I18nProvider>,
    );
    expect(content.contentVersion).toBe("test-content");
    fireEvent.click(screen.getByRole("button", { name: "Russian" }));
    fireEvent.click(screen.getByRole("button", { name: "Dark" }));
    fireEvent.click(screen.getByRole("button", { name: /Laboratory/ }));
    expect(locale).toBe("ru");
    expect(selectedTheme).toBe("dark");
    expect(selectedSkin).toBe("laboratory");
    expect(screen.getByText("Profile and settings")).toBeTruthy();
    expect(screen.getByText("Checking offline storage protection...")).toBeTruthy();
  });
});
