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

import { I18nProvider } from "../i18n";
import type { NavigationPosition } from "../prototype/layout";
import type { ThemeName } from "../theme/tokens";
import { AppShell, type AppRoute } from "./AppShell";

afterEach(cleanup);

function renderShell(
  navigationPosition: NavigationPosition,
  onNavigate: (route: AppRoute) => void = () => undefined,
  active: AppRoute = "today",
  onThemeChange: (theme: ThemeName) => void = () => undefined,
  learnerLabel = "Learner",
) {
  return render(
    <I18nProvider initialLocale="en">
      <AppShell
        active={active}
        onNavigate={onNavigate}
        learnerLabel={learnerLabel}
        locale="en"
        locales={["en", "ru", "uz"]}
        onLocaleChange={() => undefined}
        theme="deep-umber"
        onThemeChange={onThemeChange}
        navigationPosition={navigationPosition}
        lessonAvailable
        companion="iella"
      >
        <div>Route content</div>
      </AppShell>
    </I18nProvider>,
  );
}

describe("adaptive studio shell", () => {
  it("lists Notes as a study destination in the rail and on the dock", () => {
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
      "Vocabulary",
      "Library",
      "Notes",
      "Graph",
      "Practice",
      "Progress",
    ]);
    // Five equal slots keep every label whole at 360 px.
    expect(mobileItems.map((item) => item.textContent)).toEqual([
      "Today",
      "Vocabulary",
      "Library",
      "Notes",
      "More",
    ]);
    expect(screen.queryByText("Settings")).toBeNull();
    expect(container.querySelector(".learner-settings-button")).toBeNull();

    fireEvent.click(within(mobileNav as HTMLElement).getByRole("button", { name: "Notes" }));
    expect(routes).toEqual(["notes"]);
  });

  it("opens the rest of the sections from the More sheet", () => {
    const routes: AppRoute[] = [];
    const { container } = renderShell("left", (route) => routes.push(route), "graph");
    const more = within(container.querySelector(".mobile-nav") as HTMLElement).getByRole(
      "button",
      { name: "More" },
    );
    // The current section lives in the sheet, so More shows as active.
    expect(more.classList.contains("active")).toBe(true);
    expect(more.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(more);
    const sheet = screen.getByRole("navigation", { name: "More sections" });
    expect(
      within(sheet)
        .getAllByRole("button")
        .map((item) => item.textContent),
    ).toEqual(["Graph", "Practice", "Progress", "Profile"]);
    expect(within(sheet).getByRole("button", { name: "Graph" }).getAttribute("aria-current")).toBe(
      "page",
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("navigation", { name: "More sections" })).toBeNull();
    fireEvent.click(more);
    fireEvent.click(
      within(screen.getByRole("navigation", { name: "More sections" })).getByRole("button", {
        name: "Profile",
      }),
    );
    expect(routes).toEqual(["profile"]);
    expect(screen.queryByRole("navigation", { name: "More sections" })).toBeNull();
  });

  it("traps focus in the More sheet and returns it to the button", async () => {
    renderShell("left");
    const more = screen.getByRole("button", { name: "More" });
    more.focus();
    fireEvent.click(more);

    const sheet = screen.getByRole("dialog", { name: "More sections" });
    const items = within(sheet).getAllByRole("button");
    const first = items[0];
    const last = items[items.length - 1];
    if (!first || !last) {
      throw new Error("More sheet has no items");
    }
    await waitFor(() => expect(document.activeElement).toBe(first));

    last.focus();
    fireEvent.keyDown(sheet, { key: "Tab" });
    expect(document.activeElement).toBe(first);

    fireEvent.keyDown(sheet, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);

    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "More sections" })).toBeNull();
    expect(document.activeElement).toBe(more);
  });

  it("closes the More sheet on an outside click and returns focus", () => {
    renderShell("left");
    const more = screen.getByRole("button", { name: "More" });
    fireEvent.click(more);
    expect(screen.getByRole("dialog", { name: "More sections" })).toBeTruthy();

    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("dialog", { name: "More sections" })).toBeNull();
    expect(document.activeElement).toBe(more);
  });

  it("dims the page behind the wide More popover and closes on the scrim", () => {
    const { container } = renderShell("bottom");
    const more = screen.getByRole("button", { name: "More" });
    fireEvent.click(more);
    const scrim = container.querySelector(".more-scrim");
    expect(scrim).toBeTruthy();
    if (!scrim) {
      return;
    }
    fireEvent.click(scrim);
    expect(screen.queryByRole("dialog", { name: "More sections" })).toBeNull();
    expect(document.activeElement).toBe(more);
  });

  it("uses a neutral learner name when no display name is set", () => {
    const { container } = renderShell("left", undefined, "today", undefined, "");
    expect(
      container.querySelector(".learner-copy strong")?.textContent,
    ).toBe("Learner");
    expect(container.textContent).not.toMatch(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}/i,
    );
  });

  it("keeps a compact palette menu with all palettes and no slug labels", () => {
    const selected: ThemeName[] = [];
    const { container } = renderShell(
      "left",
      () => undefined,
      "today",
      (theme) => selected.push(theme),
    );
    expect(container.querySelectorAll(".palette-choice")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "Skin" })).toBeNull();
    const trigger = screen.getByRole("button", {
      name: "Theme: Deep Umber",
    });
    expect(trigger.textContent).not.toContain("burgundy");
    fireEvent.click(trigger);
    const palette = screen.getByRole("listbox", { name: "Theme" });
    expect(within(palette).getAllByRole("option")).toHaveLength(5);
    expect(
      within(palette).queryByText("burgundy", { exact: true }),
    ).toBeNull();
    fireEvent.click(
      within(palette).getByRole("option", { name: "Cream Paper" }),
    );
    expect(selected).toEqual(["cream-paper"]);
  });

  it("marks the bottom learner card active on the Profile route", () => {
    const { container } = renderShell("left", () => undefined, "profile");
    expect(
      container.querySelector(".learner-profile-button")?.classList.contains("active"),
    ).toBe(true);
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
