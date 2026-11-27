// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import { initialStatus } from "../prototype/status";
import { ProfileScreen, type ProfileScreenProps } from "./ProfileScreen";
import { SettingsScreen } from "./SettingsScreen";
import { ProgressScreen } from "./ProgressScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

const KC = "en.gram.article.a-an-sound";
const MISCONCEPTION = "m.active";
const stateJson = JSON.stringify({
  kcs: {
    [KC]: {
      ever_mastered: true,
      observations: 4,
      card: { stability: 12, difficulty: 5, last_review: 1_700_000_000_000 },
    },
  },
  misconceptions: {
    [MISCONCEPTION]: {
      alpha: 2,
      beta: 2,
      manifested: 3,
      updated_at: 1_700_000_000_000,
    },
    "m.cured": {
      alpha: 0.5,
      beta: 8.5,
      manifested: 1,
      updated_at: 1_700_000_000_000,
    },
  },
});

function profileProps(): ProfileScreenProps {
  return {
    learnerLabel: "Learner",
    learnerId: "learner-42",
    authMethod: "email",
    content: testContent(),
    stateJson,
    streak: 6,
    isAdmin: false,
    locale: "en",
    locales: ["en", "ru", "uz"],
    onLocaleChange: () => undefined,
    theme: "deep-umber",
    onThemeChange: () => undefined,
    skin: "living-studio",
    onSkinChange: () => undefined,
    navigationPosition: "left",
    onNavigationPositionChange: () => undefined,
    status: initialStatus(true),
    rejected: [],
    onExportCache: async () => undefined,
    onResetCache: async () => undefined,
    adminApi: null,
    onLogout: () => undefined,
  };
}

describe("statistics, profile, and settings", () => {
  it("renders plain-language memory metrics and the analytical charts", () => {
    render(
      <I18nProvider initialLocale="en">
        <ProgressScreen content={testContent()} stateJson={stateJson} />
      </I18nProvider>,
    );

    expect(screen.getByText("Memory retention")).toBeTruthy();
    expect(
      screen.queryByText("R(t) = (1 + t / (9 · S))⁻¹"),
    ).toBeNull();
    const strengthMetric = screen.getByText("Memory strength").parentElement;
    expect(strengthMetric).not.toBeNull();
    if (strengthMetric) {
      expect(within(strengthMetric).getByText("12")).toBeTruthy();
    }
    const complexityMetric = screen.getByText("Topic complexity").parentElement;
    expect(complexityMetric).not.toBeNull();
    if (complexityMetric) {
      expect(within(complexityMetric).getByText("5.0")).toBeTruthy();
    }
    expect(screen.getByText("Average review gap")).toBeTruthy();
    expect(screen.getByText("Mastered rules")).toBeTruthy();
    expect(screen.getByRole("img", { name: "Forgetting curve" })).toBeTruthy();
    expect(screen.getByText("Progress by CEFR band")).toBeTruthy();
    expect(screen.getByText("Domain distribution")).toBeTruthy();
    expect(screen.getByText("Misconceptions")).toBeTruthy();
    expect(screen.getByText("Cure rate 50%")).toBeTruthy();
    expect(screen.getByText("Forecast accuracy")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: /A1: 1 of 1 mastered/ }),
    ).toBeTruthy();
    expect(document.querySelector(".ui-badge")).toBeNull();
    expect(document.querySelectorAll(".status-meta").length).toBeGreaterThan(0);
  });

  it("embeds public-profile preview, visibility, and private learner cards", () => {
    render(
      <I18nProvider initialLocale="en">
        <ProfileScreen {...profileProps()} />
      </I18nProvider>,
    );

    expect(screen.getByRole("heading", { name: "Learner" })).toBeTruthy();
    expect(screen.getByText("learner-42")).toBeTruthy();
    expect(screen.getByText("Learner", { selector: ".status-meta" })).toBeTruthy();
    expect(screen.getByText("Milestones")).toBeTruthy();
    expect(screen.getByText("Level progress")).toBeTruthy();
    expect(screen.getByText("Public profile")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Error notebook" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Quick notes" })).toBeTruthy();
    for (const label of ["Self", "Team", "Friends", "Everyone"]) {
      expect(screen.getByRole("button", { name: label })).toBeTruthy();
    }
    expect(screen.queryByRole("tab")).toBeNull();
    expect(document.querySelector(".ui-badge")).toBeNull();
    expect(document.querySelectorAll(".status-meta").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Everyone" }));
    fireEvent.click(screen.getByRole("button", { name: "Preview visitor view" }));
    expect(
      screen.getByText("Visitor preview. Private notes and errors are hidden."),
    ).toBeTruthy();
    expect(screen.queryByText("learner-42")).toBeNull();
    expect(screen.queryByRole("heading", { name: "Error notebook" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Quick notes" })).toBeNull();
    expect(screen.getByRole("heading", { name: "Milestones" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Exit preview" }));
    expect(screen.getByRole("heading", { name: "Error notebook" })).toBeTruthy();
    expect(screen.getByText("learner-42")).toBeTruthy();
  });

  it("opens the embedded settings tabs from the Profile header", () => {
    render(
      <I18nProvider initialLocale="en">
        <ProfileScreen {...profileProps()} />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    for (const label of ["Appearance", "Account & Sync", "Offline & Data"]) {
      expect(screen.getByRole("tab", { name: label })).toBeTruthy();
    }
    expect(screen.queryByRole("tab", { name: "Admin" })).toBeNull();
    fireEvent.click(screen.getByRole("tab", { name: "Account & Sync" }));
    expect(screen.getByText("Learner identity")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to profile" }));
    expect(screen.getByRole("heading", { name: "Learner" })).toBeTruthy();
  });

  it("switches language, palette, skin, navigation, and sign-out from settings", () => {
    let locale = "en";
    let selectedPalette = "cream-paper";
    let selectedSkin = "living-studio";
    let navigation: NavigationPosition = "left";
    let signedOut = false;
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
          authMethod="email"
          learnerId="learner-42"
          onLogout={() => {
            signedOut = true;
          }}
        />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("tab", { name: "Account & Sync" }));
    fireEvent.click(screen.getByRole("button", { name: "Russian" }));
    expect(screen.getByText("learner-42")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    expect(signedOut).toBe(true);
    fireEvent.click(screen.getByRole("tab", { name: "Appearance" }));
    fireEvent.click(screen.getByRole("button", { name: /Sand & Forest/ }));
    fireEvent.click(screen.getByRole("button", { name: /Laboratory/ }));
    fireEvent.click(screen.getByRole("button", { name: /Neo-brutalism/ }));
    fireEvent.click(screen.getByRole("button", { name: "Right rail" }));
    expect(locale).toBe("ru");
    expect(selectedPalette).toBe("sand-forest");
    expect(selectedSkin).toBe("neo-brutalism");
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
    expect(screen.getByRole("tab", { name: "Account & Sync" })).toBeTruthy();
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
