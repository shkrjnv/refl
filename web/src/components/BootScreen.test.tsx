// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { BootScreen } from "./BootScreen";

afterEach(cleanup);

describe("boot screen", () => {
  it("shows the studio, companion, and calm progress copy", () => {
    const { container } = render(
      <I18nProvider initialLocale="en">
        <BootScreen companion="iella" />
      </I18nProvider>,
    );
    expect(screen.getByRole("status")).toBeTruthy();
    expect(screen.getByText("Preparing your studio…")).toBeTruthy();
    expect(container.querySelector(".boot-brand .brand-logo")).not.toBeNull();
    expect(container.querySelector(".boot-companion svg")).not.toBeNull();
    expect(container.querySelector(".boot-progress span")).not.toBeNull();
    expect(container.textContent).not.toContain("bundle");
  });
});
