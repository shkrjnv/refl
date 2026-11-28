// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { CompanionPicker } from "./CompanionPicker";

afterEach(cleanup);

function renderPicker(unlocked: boolean) {
  const onSelect = vi.fn();
  const onPreview = vi.fn();
  render(
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
  return { onSelect, onPreview };
}

describe("companion registry UI", () => {
  it("selects Sendo once the library milestone is open", () => {
    const { onSelect } = renderPicker(true);
    expect(screen.getByText("Sendo")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Travel together" }));
    expect(onSelect).toHaveBeenCalledWith("sendo");
  });

  it("offers a preview instead of selection while Sendo is locked", () => {
    const { onSelect, onPreview } = renderPicker(false);
    expect(
      screen.getByText("Opens with a library milestone"),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    expect(onPreview).toHaveBeenCalled();
    expect(onSelect).not.toHaveBeenCalled();
  });
});
