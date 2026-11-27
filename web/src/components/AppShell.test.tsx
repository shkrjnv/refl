// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import { AppShell, type AppRoute } from "./AppShell";

afterEach(cleanup);

function renderShell(
  navigationPosition: NavigationPosition,
  onNavigate: (route: AppRoute) => void = () => undefined,
) {
  return render(
    <I18nProvider initialLocale="en">
      <AppShell
        active="today"
        onNavigate={onNavigate}
        learnerLabel="Learner"
        locale="en"
        locales={["en", "ru", "uz"]}
        onLocaleChange={() => undefined}
        theme="deep-umber"
        onThemeChange={() => undefined}
        skin="living-studio"
        onSkinChange={() => undefined}
        navigationPosition={navigationPosition}
        lessonAvailable
      >
        <div>Route content</div>
      </AppShell>
    </I18nProvider>,
  );
}

describe("adaptive studio shell", () => {
  it("exposes exactly Today, Practice, Progress, and Profile in both navigations", () => {
    const routes: AppRoute[] = [];
    const { container } = renderShell("left", (route) => routes.push(route));

    const sideNav = container.querySelector(".side-nav");
    const mobileNav = container.querySelector(".mobile-nav");
    expect(sideNav).toBeTruthy();
    expect(mobileNav).toBeTruthy();
    if (!sideNav || !mobileNav) {
      return;
    }
    const sideItems = within(sideNav as HTMLElement).getAllByRole("button");
    const mobileItems = within(mobileNav as HTMLElement).getAllByRole("button");
    expect(sideItems.map((item) => item.textContent)).toEqual([
      "Today",
      "Practice",
      "Progress",
      "Profile",
    ]);
    expect(mobileItems.map((item) => item.textContent)).toEqual([
      "Today",
      "Practice",
      "Progress",
      "Profile",
    ]);
    expect(screen.queryByText("Settings")).toBeNull();
    expect(container.querySelector(".learner-settings-button")).toBeNull();

    fireEvent.click(within(sideNav as HTMLElement).getByRole("button", { name: "Profile" }));
    expect(routes).toEqual(["profile"]);
  });

  it("keeps the identity card as a Profile shortcut without a settings gear", () => {
    const routes: AppRoute[] = [];
    renderShell("left", (route) => routes.push(route));

    fireEvent.click(screen.getByRole("button", { name: /Learner/ }));
    expect(routes).toEqual(["profile"]);
  });

  it("docks the primary navigation left, right, or bottom", () => {
    for (const position of ["left", "right", "bottom"] as const) {
      const { container, unmount } = renderShell(position);
      const shell = container.querySelector(".studio-shell");
      expect(shell?.getAttribute("data-nav-position")).toBe(position);
      unmount();
    }
  });
});
