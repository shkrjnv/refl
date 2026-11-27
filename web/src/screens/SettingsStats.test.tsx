// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import { initialStatus } from "../prototype/status";
import { ProfileScreen } from "./ProfileScreen";
import { SettingsScreen } from "./SettingsScreen";
import { StatsScreen } from "./StatsScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

const KC = "en.gram.article.a-an-sound";
const stateJson = JSON.stringify({
  kcs: {
    [KC]: {
      ever_mastered: true,
      observations: 4,
      card: { stability: 12, difficulty: 5, last_review: 1_700_000_000_000 },
    },
  },
});

describe("statistics, profile, and settings", () => {
  it("opens the source explanation behind a statistic", async () => {
    render(
      <I18nProvider initialLocale="en">
        <StatsScreen content={testContent()} stateJson={stateJson} />
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

  it("keeps Profile separate from the compact Settings surface", () => {
    render(
      <I18nProvider initialLocale="en">
        <ProfileScreen
          learnerLabel="Learner"
          learnerId="learner-42"
          authMethod="email"
          content={testContent()}
          stateJson={stateJson}
          streak={6}
          isAdmin={false}
          onOpenSettings={() => undefined}
          onLogout={() => undefined}
        />
      </I18nProvider>,
    );

    expect(screen.getByRole("heading", { name: "Learner" })).toBeTruthy();
    expect(screen.getByText("learner-42")).toBeTruthy();
    expect(screen.getByText("Learner", { selector: ".ui-badge" })).toBeTruthy();
    expect(screen.getByText("Milestones")).toBeTruthy();
    expect(screen.getByText("Account and linked methods")).toBeTruthy();
    expect(screen.queryByRole("tab")).toBeNull();
  });

  it("switches language, palette, skin, and navigation from settings", () => {
    let locale = "en";
    let selectedPalette = "cream-paper";
    let selectedSkin = "living-studio";
    let navigation: NavigationPosition = "left";
    render(
      <I18nProvider initialLocale="en">
        <SettingsScreen
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={(next) => {
            locale = next;
          }}
          theme="cream-paper"
          onThemeChange={(next) => {
            selectedPalette = next;
          }}
          skin="living-studio"
          onSkinChange={(next) => {
            selectedSkin = next;
          }}
          navigationPosition={navigation}
          onNavigationPositionChange={(next) => {
            navigation = next;
          }}
          status={initialStatus(true)}
          rejected={[]}
          onExportCache={async () => undefined}
          onResetCache={async () => undefined}
          isAdmin={false}
          adminApi={null}
        />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("tab", { name: "Language" }));
    fireEvent.click(screen.getByRole("button", { name: "Russian" }));
    fireEvent.click(screen.getByRole("tab", { name: "Appearance" }));
    fireEvent.click(screen.getByRole("button", { name: /Sand & Forest/ }));
    fireEvent.click(screen.getByRole("button", { name: /Laboratory/ }));
    fireEvent.click(screen.getByRole("button", { name: "Right rail" }));
    expect(locale).toBe("ru");
    expect(selectedPalette).toBe("sand-forest");
    expect(selectedSkin).toBe("laboratory");
    expect(navigation).toBe("right");
    expect(screen.getByText("Settings")).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: "Offline & Data" }));
    expect(screen.getByText("Checking offline storage protection...")).toBeTruthy();
  });

  it("switches settings tabs and keeps the admin tab role-gated", () => {
    const props = {
      locale: "en" as const,
      locales: ["en", "ru", "uz"] as const,
      onLocaleChange: () => undefined,
      theme: "deep-umber" as const,
      onThemeChange: () => undefined,
      skin: "living-studio" as const,
      onSkinChange: () => undefined,
      navigationPosition: "left" as const,
      onNavigationPositionChange: () => undefined,
      status: initialStatus(true),
      rejected: [],
      onExportCache: async () => undefined,
      onResetCache: async () => undefined,
      adminApi: null,
    };
    const { rerender } = render(
      <I18nProvider initialLocale="en">
        <SettingsScreen {...props} isAdmin={false} />
      </I18nProvider>,
    );
    expect(screen.queryByRole("tab", { name: "Admin" })).toBeNull();
    expect(screen.queryByRole("tab", { name: "Account & Sync" })).toBeNull();
    fireEvent.click(screen.getByRole("tab", { name: "Offline & Data" }));
    expect(screen.getByRole("button", { name: "Export JSON" })).toBeTruthy();

    rerender(
      <I18nProvider initialLocale="en">
        <SettingsScreen {...props} isAdmin />
      </I18nProvider>,
    );
    fireEvent.click(screen.getByRole("tab", { name: "Admin" }));
    expect(screen.getByText("Registration mode")).toBeTruthy();
  });
});
