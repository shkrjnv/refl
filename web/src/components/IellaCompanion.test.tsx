// @vitest-environment jsdom

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { IellaCompanion } from "./IellaCompanion";

const rect = {
  top: 100,
  left: 100,
  width: 120,
  height: 24,
  right: 220,
  bottom: 124,
  x: 100,
  y: 100,
  toJSON: () => ({}),
};

beforeEach(() => {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue(rect);
  vi.spyOn(window, "matchMedia").mockReturnValue({
    matches: true,
  } as MediaQueryList);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("Iella companion", () => {
  it("places itself at a visible perch and reports a calm mood", () => {
    const { container } = render(
      <I18nProvider initialLocale="en">
        <div>
          <div data-iella-perch="route" data-iella-route="today" />
          <IellaCompanion activeRoute="today" />
        </div>
      </I18nProvider>,
    );
    const companion = container.querySelector(".iella-companion");
    expect(companion?.classList.contains("placed")).toBe(true);
    expect(companion?.getAttribute("data-mood")).toBe("perched");
    expect((companion as HTMLElement | null)?.style.transform).toContain(
      "translate3d",
    );
  });
});
