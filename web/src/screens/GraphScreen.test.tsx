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

function renderGraph(content = testContent()) {
  const onPractice = vi.fn();
  render(
    <I18nProvider initialLocale="en">
      <GraphScreen
        content={content}
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
    expect(node.querySelector(".graph-node-head .graph-node-title")).not.toBeNull();
    expect(node.querySelector(".graph-node-head .graph-node-dot")).not.toBeNull();
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

describe("knowledge graph stage", () => {
  it("places cards in the same pixel space as the edge layer", () => {
    renderGraph();
    const world = document.querySelector<HTMLElement>(".graph-world");
    const svg = document.querySelector("svg.graph-svg");
    expect(world).not.toBeNull();
    expect(svg?.getAttribute("viewBox")).toBe(
      `0 0 ${parseFloat(world?.style.width ?? "0")} ${parseFloat(world?.style.height ?? "0")}`,
    );
    expect(svg?.getAttribute("preserveAspectRatio")).toBeNull();
    for (const card of document.querySelectorAll<HTMLElement>(".graph-node")) {
      expect(card.style.left).toMatch(/px$/);
      expect(card.style.width).toMatch(/px$/);
      expect(parseFloat(card.style.height)).toBeGreaterThanOrEqual(44);
    }
  });

  it("zooms with the controls and fits back", () => {
    renderGraph();
    const world = document.querySelector<HTMLElement>(".graph-world");
    const before = world?.style.transform;
    fireEvent.click(screen.getByRole("button", { name: "Zoom in" }));
    expect(world?.style.transform).not.toBe(before);
    expect(world?.style.transform).toContain("scale(1.2)");
    fireEvent.click(screen.getByRole("button", { name: "Fit" }));
    expect(world?.style.transform).toBe(before);
  });

  it("walks back through visited topics and clears with Escape", () => {
    renderGraph(testContent({ x: 500, y: 1000 }, { neighbour: true }));
    const nodes = [...document.querySelectorAll<HTMLElement>(".graph-node")];
    expect(nodes.length).toBeGreaterThan(1);
    fireEvent.click(nodes[0] as HTMLElement);
    fireEvent.click(nodes[1] as HTMLElement);
    const back = screen.getByRole("button", { name: /^Back to / });
    fireEvent.click(back);
    expect(nodes[0]?.getAttribute("aria-pressed")).toBe("true");
    expect(document.querySelectorAll(".graph-edge-requires").length).toBeGreaterThan(0);
    fireEvent.keyDown(nodes[0] as HTMLElement, { key: "Escape" });
    expect(screen.getByText("Select a topic to open its details.")).toBeTruthy();
  });
});

describe("knowledge graph zero layout", () => {
  it("uses the generated fallback instead of stacking at zero", () => {
    renderGraph(testContent({ x: 0, y: 0 }));
    expect(
      screen.getByText(
        "This bundle has no saved graph positions, so the map uses a generated level layout.",
      ),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "a or an, Learning" }),
    ).toBeTruthy();
  });
});
