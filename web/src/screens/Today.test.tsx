// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { activitySummary, type ActivitySummary } from "../prototype/activity";
import {
  defaultLayout,
  type DashboardLayout,
} from "../prototype/layout";
import { initialStatus } from "../prototype/status";
import { TodayScreen } from "./TodayScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

const KC = "en.gram.article.a-an-sound";

function activityWithAnswers(): ActivitySummary {
  const days = Array.from({ length: 49 }, (_, index) => ({
    day: `2026-01-${String(index + 1).padStart(2, "0")}`,
    count: index === 48 ? 2 : index === 47 ? 1 : 0,
    weekday: index % 7,
  }));
  return { days, total: 3, bestStreak: 2, todayCount: 2 };
}

function renderToday(
  status = initialStatus(true),
  stateJson: string | null = null,
  onLayoutChange: (layout: DashboardLayout) => void = () => undefined,
  onResetLayout: () => void = () => undefined,
  activity: ActivitySummary = activityWithAnswers(),
) {
  render(
    <I18nProvider initialLocale="en">
      <TodayScreen
        content={testContent()}
        stateJson={stateJson}
        learnerLabel="Learner"
        streak={6}
        status={status}
        activity={activity}
        layout={defaultLayout()}
        onLayoutChange={onLayoutChange}
        onResetLayout={onResetLayout}
        onStart={() => undefined}
        onOpenStats={() => undefined}
        tourOpen={false}
        onTourComplete={() => undefined}
      />
    </I18nProvider>,
  );
}

describe("Today clean state", () => {
  it("shows no synced badge, storage diagnostics, or normal status chip", () => {
    renderToday({
      ...initialStatus(true),
      lastSyncedAt: Date.now(),
      persistence: "granted",
    });

    expect(screen.queryByText("Everything is synced.")).toBeNull();
    expect(screen.queryByText("Offline storage is persistent.")).toBeNull();
    expect(
      screen.queryByText("This browser cannot protect offline storage."),
    ).toBeNull();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByText("Five rules, one route")).toBeTruthy();
    expect(screen.getByText("6 day streak")).toBeTruthy();
  });

  it("shows a compact cue only when offline or work is pending", () => {
    renderToday({
      ...initialStatus(false),
      pendingOutbox: 3,
      persistence: "denied",
    });

    expect(screen.getByText("Offline. New answers stay on this device.")).toBeTruthy();
    expect(screen.getByText("3 answers waiting to sync.")).toBeTruthy();
    expect(
      screen.queryByText(
        /The browser may clear offline data/,
      ),
    ).toBeNull();
  });

  it("renders FSRS memory, focus, and seven-week activity from learner state", () => {
    renderToday(
      initialStatus(true),
      JSON.stringify({
        kcs: {
          [KC]: {
            ever_mastered: true,
            observations: 4,
            card: {
              stability: 12,
              difficulty: 5,
              last_review: Date.now() - 20 * 86_400_000,
            },
          },
        },
      }),
    );

    expect(screen.getByText("FSRS memory")).toBeTruthy();
    expect(screen.getByText("1 due")).toBeTruthy();
    expect(screen.getByText("Misconceptions and focus")).toBeTruthy();
    expect(screen.getByText("Seven weeks")).toBeTruthy();
    expect(screen.getByText("Today: 2 answers")).toBeTruthy();
    expect(screen.getByRole("img", { name: /answers in seven weeks/ })).toBeTruthy();
  });

  it("shows explicit empty states before any learner activity", () => {
    renderToday(
      initialStatus(true),
      null,
      () => undefined,
      () => undefined,
      activitySummary([], null, Date.now()),
    );
    expect(
      screen.getByText("No FSRS memory has started yet. Finish a session to start the ring."),
    ).toBeTruthy();
    expect(
      screen.getByText("No weak spots yet. Your first answers will surface them here."),
    ).toBeTruthy();
    expect(
      screen.getByText("No activity in the last seven weeks yet."),
    ).toBeTruthy();
  });

  it("enters a drag-and-drop layout editing mode", () => {
    renderToday();
    fireEvent.click(screen.getByRole("button", { name: "Edit layout" }));
    expect(screen.getByRole("button", { name: "Done" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reset layout" })).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Move Five rules, one route down" }),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Drag Articles in context" }),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Resize FSRS memory" }),
    ).toBeTruthy();
  });
  it("reorders, resizes, and resets widgets through accessible controls", () => {
    const changes: DashboardLayout[] = [];
    let reset = false;
    renderToday(
      initialStatus(true),
      null,
      (layout) => {
        changes.push(layout);
      },
      () => {
        reset = true;
      },
    );

    fireEvent.click(screen.getByRole("button", { name: "Edit layout" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move Five rules, one route down" }),
    );
    expect(changes[0]?.order).toEqual([
      "hero",
      "memory",
      "rules",
      "misconceptions",
      "activity",
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Reset layout" }));
    expect(reset).toBe(true);
  });

  it("resizes a widget through its pointer resize handle", () => {
    const changes: DashboardLayout[] = [];
    renderToday(initialStatus(true), null, (layout) => {
      changes.push(layout);
    });
    fireEvent.click(screen.getByRole("button", { name: "Edit layout" }));
    const handle = screen.getByRole("button", { name: "Resize FSRS memory" });
    fireEvent.pointerDown(handle, { pointerId: 7, clientX: 10, clientY: 10 });
    fireEvent.pointerMove(window, { pointerId: 7, clientX: 280, clientY: 10 });
    expect(changes.some((layout) => layout.colSpan.memory === 3)).toBe(true);
  });
});
