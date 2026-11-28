// @vitest-environment jsdom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { CompanionPicker } from "./CompanionPicker";

afterEach(cleanup);

function renderPicker(unlocked: boolean) {
  const onSelect = vi.fn();
  const onPreview = vi.fn();
  const { container } = render(
    <I18nProvider initialLocale="en">
      <CompanionPicker
        companion="iella"
        sendoUnlocked={unlocked}
        sendoPreview={false}
        onSelect={onSelect}
        onPreviewSendo={onPreview}
      />
    </I18nProvider>,
  );
  return { onSelect, onPreview, container };
}

describe("companion registry UI", () => {
  it("selects Sendo once the library milestone is open", () => {
    const { onSelect } = renderPicker(true);
    expect(screen.getByText("Sendo")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Travel together" }));
    expect(onSelect).toHaveBeenCalledWith("sendo");
  });

  it("marks only the selected companion as the learner's companion", () => {
    const { container } = renderPicker(true);
    const iella = container.querySelector('[data-companion="iella"]');
    const sendo = container.querySelector('[data-companion="sendo"]');
    expect(iella).toBeTruthy();
    expect(sendo).toBeTruthy();
    if (!iella || !sendo) {
      return;
    }
    expect(within(iella as HTMLElement).getByText("Your companion")).toBeTruthy();
    expect(
      within(sendo as HTMLElement).queryByText("Your companion"),
    ).toBeNull();
    expect(
      within(sendo as HTMLElement).getByText("Library keeper"),
    ).toBeTruthy();
  });

  it("offers a preview instead of selection while Sendo is locked", () => {
    const { onSelect, onPreview, container } = renderPicker(false);
    const sendo = container.querySelector('[data-companion="sendo"]');
    expect(sendo).toBeTruthy();
    if (!sendo) {
      return;
    }
    expect(
      within(sendo as HTMLElement).getByText("Opens with a library milestone"),
    ).toBeTruthy();
    expect(
      within(sendo as HTMLElement).queryByText("Ready to travel"),
    ).toBeNull();
    expect(screen.getAllByText("Ready to travel")).toHaveLength(1);
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    expect(onPreview).toHaveBeenCalled();
    expect(onSelect).not.toHaveBeenCalled();
  });
});
