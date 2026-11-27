// @vitest-environment jsdom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { initialStatus } from "../prototype/status";
import { TodayScreen } from "./TodayScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

function renderToday(status = initialStatus(true)) {
  render(
    <I18nProvider initialLocale="en">
      <TodayScreen
        content={testContent()}
        stateJson={null}
        learnerLabel="Learner"
        status={status}
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
});
