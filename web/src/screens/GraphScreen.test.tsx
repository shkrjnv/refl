// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import type { RecordStore } from "../prototype/records";
import { GraphScreen } from "./GraphScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

const state = JSON.stringify({
  kcs: {
    "en.gram.article.a-an-sound": {
      observations: 6,
      ever_mastered: false,
      card: {
        stability: 4,
        difficulty: 3,
        last_review: Date.now() - 86_400_000,
      },
    },
  },
  misconceptions: {},
});

function fakeStore(): RecordStore {
  return {
    saveNote: vi.fn(async () => state),
    deleteNote: vi.fn(async () => state),
    saveCustomWord: vi.fn(async () => state),
    archiveCustomWord: vi.fn(async () => state),
    reviewLexeme: vi.fn(async () => state),
    createBookMark: vi.fn(async () => ({ stateJson: state, markId: "m" })),
    deleteBookMark: vi.fn(async () => state),
    selectCompanion: vi.fn(async () => state),
  } as unknown as RecordStore;
}

function renderGraph() {
  const onPractice = vi.fn();
  render(
    <I18nProvider initialLocale="en">
      <GraphScreen
        content={testContent()}
        stateJson={state}
        locale="en"
        recordsWriter={fakeStore()}
        onStateChange={() => undefined}
        onPractice={onPractice}
      />
    </I18nProvider>,
  );
  return onPractice;
}

describe("knowledge graph", () => {
  it("opens a topic drawer with model numbers and practice", () => {
    const onPractice = renderGraph();
    const node = screen.getByRole("button", {
      name: "a or an, Learning",
    });
    fireEvent.click(node);
    expect(screen.getByText("Why this number")).toBeTruthy();
    expect(screen.getByText("Answers")).toBeTruthy();
    expect(screen.getByText("6")).toBeTruthy();
    expect(screen.getByText("4.0d")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Practice this topic" }));
    expect(onPractice).toHaveBeenCalledWith("en.gram.article.a-an-sound");
  });

  it("switches between the layered graph and the web explorer", () => {
    renderGraph();
    const layered = screen.getByRole("button", { name: "Layered" });
    const web = screen.getByRole("button", { name: "Web" });
    expect(layered.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(web);
    expect(web.getAttribute("aria-pressed")).toBe("true");
    expect(layered.getAttribute("aria-pressed")).toBe("false");
  });
});
