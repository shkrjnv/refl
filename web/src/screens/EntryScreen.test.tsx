// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { EntryScreen } from "./EntryScreen";

afterEach(cleanup);

function renderEntry(overrides: Partial<Parameters<typeof EntryScreen>[0]> = {}) {
  const props = {
    placementAvailable: true,
    busy: false,
    error: null,
    onStartFromZero: vi.fn(),
    onCheck: vi.fn(),
    ...overrides,
  };
  render(
    <I18nProvider initialLocale="en">
      <EntryScreen {...props} />
    </I18nProvider>,
  );
  return props;
}

describe("entry screen", () => {
  it("offers a zero start and a placement check", () => {
    const props = renderEntry();
    expect(screen.getByText("Where would you like to start?")).toBeTruthy();
    fireEvent.click(
      screen.getByRole("button", { name: "Start from the beginning" }),
    );
    expect(props.onStartFromZero).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Check my level" }));
    expect(props.onCheck).toHaveBeenCalledTimes(1);
  });

  it("disables the check with a note when no reviewed items exist", () => {
    renderEntry({ placementAvailable: false });
    const check = screen.getByRole("button", { name: "Check my level" });
    expect(check.hasAttribute("disabled")).toBe(true);
    expect(
      screen.getByText(
        "No reviewed questions are available for this language yet. Start from the beginning.",
      ),
    ).toBeTruthy();
  });

  it("announces an error", () => {
    renderEntry({ error: "storage unavailable" });
    expect(screen.getByRole("alert").textContent).toContain(
      "storage unavailable",
    );
  });
});
