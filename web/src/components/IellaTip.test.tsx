// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { resetSectionTips, SECTION_TIPS } from "../prototype/sectionTips";
import { IellaTip } from "./IellaTip";

afterEach(() => {
  cleanup();
  resetSectionTips();
});

describe("contextual Iella tips", () => {
  it("shows each section tip once and then stays quiet", () => {
    for (const section of SECTION_TIPS) {
      const { unmount } = render(
        <I18nProvider initialLocale="en">
          <IellaTip section={section} />
        </I18nProvider>,
      );
      expect(screen.getByRole("note")).toBeTruthy();
      expect(screen.getByRole("note").textContent).not.toBe("");
      unmount();

      const again = render(
        <I18nProvider initialLocale="en">
          <IellaTip section={section} />
        </I18nProvider>,
      );
      expect(screen.queryByRole("note")).toBeNull();
      again.unmount();
    }
  });

  it("can be dismissed without waiting for the next visit", () => {
    render(
      <I18nProvider initialLocale="en">
        <IellaTip section="graph" />
      </I18nProvider>,
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Dismiss Iella's tip" }),
    );
    expect(screen.queryByRole("note")).toBeNull();
  });
});
