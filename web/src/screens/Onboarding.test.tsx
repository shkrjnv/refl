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

  it("tracks the target with a single SVG mask and no backdrop blur", () => {
    const source = readFileSync("src/components/IellaTour.tsx", "utf8");
    const css = readFileSync("src/app.css", "utf8");
    expect(source).toContain("behavior: reduceMotion ? \"auto\" : \"smooth\"");
    expect(source).toContain("block: \"center\"");
    expect(source).toContain("new ResizeObserver");
    expect(source).toContain("requestAnimationFrame");
    expect(css).toContain(".iella-tour-mask {");
    expect(css).toContain(".iella-tour-highlight-svg {");
    expect(css).not.toMatch(/\.iella-tour-shade/);
    expect(css).not.toContain("backdrop-filter: blur(2px)");
  });

  it("offers one interactive try step before the route", () => {
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
    expect(
      screen.getByText("I have ___ cat."),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "a" }));
    expect(
      screen.getByText(/Right\. A cat: the next sound is a consonant\./),
    ).toBeTruthy();
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
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    const highlight = document.querySelector(
      ".iella-tour-highlight-svg",
    );
    expect(highlight).toBeTruthy();
    expect(highlight?.getAttribute("x")).toBe("-8");
    expect(highlight?.getAttribute("y")).toBe("-8");
    expect(highlight?.getAttribute("width")).toBe("16");
    expect(highlight?.getAttribute("height")).toBe("16");
    expect(document.querySelector(".iella-tour-mask")).toBeTruthy();
    expect(document.querySelectorAll(".iella-tour-shade")).toHaveLength(0);
    expect(document.querySelector(".iella-tour-blocker")).toBeNull();
    expect(document.querySelector(".iella-spotlight")).toBeNull();
  });
});
