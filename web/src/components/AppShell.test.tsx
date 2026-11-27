// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
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
  it("opens Profile from the learner card and Settings from the compact gear", () => {
    const routes: AppRoute[] = [];
    renderShell("left", (route) => routes.push(route));

    fireEvent.click(screen.getByRole("button", { name: /Learner/ }));
    fireEvent.click(screen.getAllByRole("button", { name: "Settings" })[0]!);
    expect(routes).toEqual(["profile", "settings"]);
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
