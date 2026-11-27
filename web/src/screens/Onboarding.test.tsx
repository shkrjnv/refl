// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { initialStatus } from "../prototype/status";
import { TodayScreen } from "./TodayScreen";
import { testContent } from "./testContent";

afterEach(() => {
  cleanup();
  window.localStorage.removeItem("refl.tour");
});

describe("Iella onboarding", () => {
  it("shows the arrival line and can be skipped in one tap", () => {
    let completed = false;
    render(
      <I18nProvider initialLocale="en">
        <TodayScreen
          content={testContent()}
          stateJson={null}
          learnerLabel="Learner"
          status={initialStatus(true)}
          onStart={() => undefined}
          onOpenStats={() => undefined}
          tourOpen
          onTourComplete={() => {
            completed = true;
          }}
        />
      </I18nProvider>,
    );

    expect(
      screen.getByText(/I am Iella\. I will show you the route/),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Skip" }));
    expect(completed).toBe(true);
  });
});
