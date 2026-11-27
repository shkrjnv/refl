// @vitest-environment jsdom

import { readFileSync } from "node:fs";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import { initialStatus } from "../prototype/status";
import { activitySummary } from "../prototype/activity";
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
          streak={4}
          status={initialStatus(true)}
          activity={activitySummary([], null)}
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

  it("stages focus with blur, smooth centering, and exact bounds", () => {
    const source = readFileSync("src/components/IellaTour.tsx", "utf8");
    const css = readFileSync("src/app.css", "utf8");
    expect(source).toContain("behavior: reduceMotion ? \"auto\" : \"smooth\"");
    expect(source).toContain("block: \"center\"");
    expect(css).toMatch(
      /\.iella-tour-shade,[\s\S]*?\.iella-tour-blocker[\s\S]*?backdrop-filter: blur\(2px\)/,
    );
  });

  it("targets the route card with exact padded bounds and no full-page spotlight", () => {
    render(
      <I18nProvider initialLocale="en">
        <TodayScreen
          content={testContent()}
          stateJson={null}
          learnerLabel="Learner"
          streak={4}
          status={initialStatus(true)}
          activity={activitySummary([], null)}
          onStart={() => undefined}
          onOpenStats={() => undefined}
          tourOpen
          onTourComplete={() => undefined}
        />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    const highlight = document.querySelector<HTMLElement>(".iella-tour-highlight");
    expect(highlight).toBeTruthy();
    expect(highlight?.style.top).toBe("-8px");
    expect(highlight?.style.left).toBe("-8px");
    expect(highlight?.style.width).toBe("16px");
    expect(highlight?.style.height).toBe("16px");
    expect(document.querySelectorAll(".iella-tour-shade")).toHaveLength(4);
    expect(document.querySelector(".iella-tour-blocker")).toBeNull();
    expect(document.querySelector(".iella-spotlight")).toBeNull();
  });
});
