// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { buildItemView } from "../prototype/item";
import type { PlacementRunner, PlacementView } from "../prototype/placement";
import { testContent } from "./testContent";
import { PlacementScreen } from "./PlacementScreen";

afterEach(cleanup);

function itemView() {
  const content = testContent();
  const entry = content.item({ id: "test.gap", version: 1 });
  if (!entry) {
    throw new Error("test item missing");
  }
  return buildItemView(content, entry.item, 0, "en", "placement");
}

function renderScreen(
  onSubmit: (response: unknown) => Promise<PlacementView>,
  onComplete = vi.fn(),
) {
  const runner = { submit: vi.fn(onSubmit) } as unknown as PlacementRunner;
  const view: PlacementView = {
    phase: "item",
    item: itemView(),
    answered: 0,
    planned: 10,
  };
  render(
    <I18nProvider initialLocale="en">
      <PlacementScreen
        runner={runner}
        initialView={view}
        onComplete={onComplete}
        onExit={() => undefined}
      />
    </I18nProvider>,
  );
  return { runner, onComplete };
}

describe("placement screen", () => {
  it("shows no hint or KC titles and advances without feedback", async () => {
    const result: PlacementView = {
      phase: "result",
      result: {
        level: { point: 1, low: 0.2, high: 1.4 },
        startLevel: 1,
        answered: 1,
        source: "diagnostic",
      },
    };
    const { runner, onComplete } = renderScreen(async () => result);
    expect(screen.queryByText("Listen to the first sound.")).toBeNull();
    expect(screen.queryByText("a or an")).toBeNull();
    fireEvent.change(screen.getByLabelText("answer"), {
      target: { value: "a" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Check" }));
    await waitFor(() => expect(runner.submit).toHaveBeenCalledTimes(1));
    expect(await screen.findByText("Your starting point")).toBeTruthy();
    expect(screen.getByText("Start at A1.")).toBeTruthy();
    expect(screen.getByText("Likely range: Pre-A1–A1.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Start learning" }));
    expect(onComplete).toHaveBeenCalledTimes(1);
  });
});
