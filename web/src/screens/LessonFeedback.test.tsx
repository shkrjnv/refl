// @vitest-environment jsdom

import { readFileSync } from "node:fs";

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { LessonRunner, LessonView } from "../prototype/lesson";
import { LessonScreen } from "./LessonScreen";

afterEach(cleanup);

const view: LessonView = {
  phase: "feedback",
  item: null,
  explanation: null,
  feedback: {
    correct: true,
    verdict: "correct",
    why: "The article agrees with the next word.",
    misconception: null,
    correctAnswer: "an advice",
  },
  summary: null,
  answered: 1,
  planned: 3,
};

describe("lesson confirmation check", () => {
  it("renders a checkmark in the correct verdict without a pulse", () => {
    const runner = {
      advanceAfterFeedback: async () => view,
    } as unknown as LessonRunner;

    render(
      <I18nProvider initialLocale="en">
        <LessonScreen
          runner={runner}
          initialView={view}
          onComplete={() => undefined}
          onExit={() => undefined}
        />
      </I18nProvider>,
    );

    const verdict = screen.getByText("Correct");
    const row = verdict.closest(".feedback-verdict");
    expect(row).not.toBeNull();
    const check = row?.querySelector(".studio-icon");
    expect(check?.getAttribute("class")).toContain("lucide-check");
    expect(check?.querySelector("path")).not.toBeNull();
    expect(document.querySelector(".pulse-confirm")).toBeNull();

    const css = readFileSync("src/app.css", "utf8");
    const verdictStart = css.lastIndexOf(".feedback-verdict {");
    const verdictRule = css.slice(
      verdictStart,
      css.indexOf("}", verdictStart) + 1,
    );
    expect(verdictRule).toContain("display: flex");
    expect(verdictRule).toContain("align-items: center");
  });
});
