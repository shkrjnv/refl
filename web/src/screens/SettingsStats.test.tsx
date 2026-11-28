// @vitest-environment jsdom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { ProfileResponse } from "../api/client";
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
    navigationPosition: "left",
    onNavigationPositionChange: () => undefined,
    status: initialStatus(true),
    rejected: [],
    minor: false,
    serverProfile: null,
    onExportCache: async () => undefined,
    onExportAccount: async () => undefined,
    onEraseAccount: async () => undefined,
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

  it("shows Pre-A1 on a profile with no evidence", () => {
    render(
      <I18nProvider initialLocale="en">
        <ProfileScreen {...profileProps()} stateJson={null} />
      </I18nProvider>,
    );
    expect(screen.getByText("Level Pre-A1 companion")).toBeTruthy();
    expect(
      screen.getByText("Your level grows after the first session."),
    ).toBeTruthy();
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
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent?.trim())).toEqual([
      "Account & Sync",
      "Offline & Data",
      "Appearance",
    ]);
    expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
    expect(screen.queryByRole("tab", { name: "Admin" })).toBeNull();
    expect(screen.getByText("Learner identity")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Sign out" })).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: "Appearance" }));
    expect(screen.getByRole("heading", { name: "Appearance" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to profile" }));
    expect(screen.getByRole("heading", { name: "Learner" })).toBeTruthy();
  });

  it("offers account export, erasure and the AGPL source link", async () => {
    let exported = 0;
    let erased = 0;
    render(
      <I18nProvider initialLocale="en">
        <ProfileScreen
          {...profileProps()}
          onExportAccount={async () => {
            exported += 1;
          }}
          onEraseAccount={async () => {
            erased += 1;
          }}
        />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    const source = screen.getByRole("link", { name: /Open source code/ });
    expect(source.getAttribute("href")).toContain("refl.uz/source");
    expect(source.textContent).toContain("AGPL-3.0-only");

    fireEvent.click(screen.getByRole("button", { name: "Download export" }));
    await waitFor(() => expect(exported).toBe(1));
    expect(screen.getByText("Account export downloaded.")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Erase account" }));
    expect(
      screen.getByRole("button", { name: "Confirm erasure" }),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Confirm erasure" }));
    await waitFor(() => expect(erased).toBe(1));
  });

  it("shows the child privacy and chat policy for a minor account", () => {
    const childProfile: ProfileResponse = {
      learner_id: "learner-42",
      public_code: "AAAA0001",
      display_name: "Learner",
      interface_lang: "en",
      minor: true,
      access_status: "active",
      created_at: 1_700_000_000_000,
      privacy: {
        profile_visibility: "private",
        friend_requests_from: "team",
        updated_at: 1_700_000_000_000,
      },
      chat: { personal_messages: false, team_chat: true },
    };
    render(
      <I18nProvider initialLocale="en">
        <ProfileScreen
          {...profileProps()}
          minor
          serverProfile={childProfile}
        />
      </I18nProvider>,
    );

    expect(screen.getByText("Child account")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Everyone" }).hasAttribute("disabled"),
    ).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByText("Child account privacy")).toBeTruthy();
    expect(
      screen.getByText(/Personal messages are off/),
    ).toBeTruthy();
  });

  it("switches language, palette, navigation, and sign-out from settings", async () => {
    let locale = "en";
    let selectedPalette = "cream-paper";
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
          navigationPosition={navigation}
          onNavigationPositionChange={(next) => {
            navigation = next;
          }}
          status={initialStatus(true)}
          rejected={[]}
          minor={false}
          serverProfile={null}
          onExportCache={async () => undefined}
          onExportAccount={async () => undefined}
          onEraseAccount={async () => undefined}
          onResetCache={async () => undefined}
          isAdmin={false}
          adminApi={null}
          authMethod="email"
          learnerId="learner-42"
          onLogout={() => {
            signedOut = true;
          }}
          onSyncNow={async () => true}
        />
      </I18nProvider>,
    );

    expect(
      screen.getByRole("tab", { name: "Account & Sync" }).getAttribute(
        "aria-selected",
      ),
    ).toBe("true");
    const sync = screen.getByRole("button", { name: "Retry sync" });
    fireEvent.click(sync);
    await waitFor(() =>
      expect(
        sync.querySelector(".crossfade-icon")?.getAttribute("data-active"),
      ).toBe("true"),
    );
    fireEvent.click(screen.getByRole("button", { name: "Russian" }));
    expect(screen.getByText("learner-42")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
    expect(signedOut).toBe(true);
    fireEvent.click(screen.getByRole("tab", { name: "Appearance" }));
    for (const name of [
      /Deep Umber/,
      /Burgundy Studio/,
      /Midnight Cobalt/,
      /Cream Paper/,
      /Sand & Forest/,
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    fireEvent.click(screen.getByRole("button", { name: /Sand & Forest/ }));
    fireEvent.click(screen.getByRole("button", { name: "Right rail" }));
    expect(locale).toBe("ru");
    expect(selectedPalette).toBe("sand-forest");
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
      navigationPosition: "left" as const,
      onNavigationPositionChange: () => undefined,
      status: initialStatus(true),
      rejected: [],
      minor: false,
      serverProfile: null,
      onExportCache: async () => undefined,
      onExportAccount: async () => undefined,
      onEraseAccount: async () => undefined,
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
    expect(
      screen.getAllByRole("tab").map((tab) => tab.textContent?.trim()),
    ).toEqual(["Account & Sync", "Offline & Data", "Appearance", "Admin"]);
    fireEvent.click(screen.getByRole("tab", { name: "Admin" }));
    expect(screen.getByText("Registration mode")).toBeTruthy();
  });
});
