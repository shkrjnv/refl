// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import {
  ACTIVITY_DAYS,
  activitySummary,
  type ActivitySummary,
} from "../prototype/activity";
import { PrototypeContent } from "../prototype/content";
import { initialStatus, type AppStatus } from "../prototype/status";
import { TodayScreen } from "./TodayScreen";

afterEach(cleanup);

const ARTICLE_IDS = [
  "en.gram.article.a-an-sound",
  "en.gram.article.the-first-mention",
  "en.gram.article.zero-article",
  "en.gram.article.generic-plural",
  "en.gram.article.unique-reference",
] as const;

function articleKc(id: string, title: string, rank: number) {
  return {
    kc: {
      id,
      subject: "en",
      domain: "grammar",
      title: { en: title, ru: title, uz: title },
      review: {
        en: { status: "reviewed" },
        ru: { status: "reviewed" },
        uz: { status: "reviewed" },
      },
      level: rank === 1 ? "A1" : "A2",
      importance: 0.9,
      rule: title,
      tags: [],
      observed_only: false,
    },
    status: "live",
    level_rank: rank,
  };
}

function bentoContent(): PrototypeContent {
  const bundle = {
    format_version: 10,
    content_version: "bento-test",
    content: {
      kcs: [
        articleKc(ARTICLE_IDS[0], "a or an", 1),
        articleKc(ARTICLE_IDS[1], "the first mention", 1),
        articleKc(ARTICLE_IDS[2], "zero article", 2),
        articleKc(ARTICLE_IDS[3], "generic plural", 2),
        articleKc(ARTICLE_IDS[4], "unique reference", 2),
      ],
      items: [],
      explanations: [],
      instructions: [],
      misconceptions: [
        {
          id: "m.active",
          title: {
            en: "Missing article",
            ru: "Пропущен артикль",
            uz: "Artikl tushib qolgan",
          },
        },
      ],
    },
  };
  return new PrototypeContent(
    JSON.stringify(bundle),
    "bento-test",
    "bento-digest",
    "",
  );
}

function learnerState(): string {
  const now = Date.now();
  return JSON.stringify({
    kcs: {
      [ARTICLE_IDS[0]]: {
        ever_mastered: true,
        observations: 6,
        card: {
          stability: 20,
          difficulty: 4,
          last_review: now - 86_400_000,
        },
      },
      [ARTICLE_IDS[1]]: {
        ever_mastered: false,
        observations: 4,
        card: {
          stability: 5,
          difficulty: 6,
          last_review: now - 10 * 86_400_000,
        },
      },
      [ARTICLE_IDS[2]]: {
        ever_mastered: false,
        observations: 2,
        card: {
          stability: 2,
          difficulty: 5,
          last_review: now - 86_400_000,
        },
      },
    },
    misconceptions: {
      "m.active": {
        alpha: 2,
        beta: 2,
        manifested: 3,
        updated_at: now,
      },
      "m.cured": {
        alpha: 0.5,
        beta: 8.5,
        manifested: 2,
        updated_at: now,
      },
    },
  });
}

function activityWithAnswers(): ActivitySummary {
  const days = Array.from({ length: ACTIVITY_DAYS }, (_, index) => ({
    day: `2026-${String(Math.floor(index / 28) + 1).padStart(2, "0")}-${String((index % 28) + 1).padStart(2, "0")}`,
    count: index === ACTIVITY_DAYS - 1 ? 2 : index === ACTIVITY_DAYS - 2 ? 1 : 0,
    weekday: index % 7,
  }));
  return { days, total: 3, bestStreak: 2, todayCount: 2 };
}

function renderToday(
  status: AppStatus = initialStatus(true),
  stateJson: string | null = learnerState(),
  activity: ActivitySummary = activityWithAnswers(),
  onStart: (minutes: number) => void = () => undefined,
) {
  render(
    <I18nProvider initialLocale="en">
      <TodayScreen
        content={bentoContent()}
        stateJson={stateJson}
        learnerLabel="Learner"
        streak={6}
        status={status}
        activity={activity}
        onStart={onStart}
        onOpenStats={() => undefined}
        tourOpen={false}
        companion="iella"
        recordsWriter={null}
        onStateChange={() => undefined}
        onTourComplete={() => undefined}
      />
    </I18nProvider>,
  );
}

describe("Today fixed bento", () => {
  it("shows the four destinations' greeting without synced diagnostics", () => {
    renderToday({
      ...initialStatus(true),
      lastSyncedAt: Date.now(),
      persistence: "granted",
    });

    expect(screen.getByText("Good to see you, Learner")).toBeTruthy();
    expect(screen.getByText("6 day streak")).toBeTruthy();
    expect(screen.queryByText("Everything is synced.")).toBeNull();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByRole("button", { name: "Edit layout" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Done" })).toBeNull();
    expect(document.querySelector(".widget-resize-handle")).toBeNull();
    expect(document.querySelector(".dashboard-widget")).toBeNull();
  });

  it("renders the photographic hero and three balanced bento rows", () => {
    renderToday();

    const hero = document.querySelector(".today-hero-banner");
    expect(hero).toBeTruthy();
    expect(hero?.getAttribute("data-tour")).toBe("route");
    expect(hero?.querySelector(".hero-illustration")).toBeTruthy();
    expect(
      hero?.querySelector(".hero-illustration")?.getAttribute("src"),
    ).toContain("hero-nature");
    expect(
      hero?.querySelector("[data-tour=\"start\"]")?.textContent,
    ).toContain("Start session");

    for (const heading of [
      "Today's route",
      "Level progress",
      "Reviews and observations",
      "Articles mastery",
      "Weekly activity",
      "Error notebook",
      "Notes and route focus",
    ]) {
      expect(screen.getByText(heading)).toBeTruthy();
    }
    expect(screen.getAllByText("Mastered").length).toBeGreaterThan(0);
    expect(screen.getByText("Fading")).toBeTruthy();
    expect(screen.getByText("Learning")).toBeTruthy();
    expect(screen.getAllByText("Not started").length).toBeGreaterThan(0);
    expect(screen.getAllByText("A1 → A2").length).toBeGreaterThan(0);
    expect(screen.getByText("×3")).toBeTruthy();
    expect(screen.queryByText("×2")).toBeNull();
    expect(screen.getAllByText("Missing article").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Review").length).toBeGreaterThan(0);
    expect(document.querySelector(".ui-badge")).toBeNull();
    expect(document.querySelector(".mastery-status")).toBeNull();
    expect(document.querySelector(".streak-badge")).toBeNull();
    expect(document.querySelector(".sync-cue")).toBeNull();
    expect(document.querySelectorAll(".status-meta").length).toBeGreaterThan(0);
  });

  it("selects 10, 15, or 25 minutes and passes the choice to the session", () => {
    const starts: number[] = [];
    renderToday(initialStatus(true), learnerState(), activityWithAnswers(), (minutes) =>
      starts.push(minutes),
    );

    const ten = screen.getByRole("button", { name: "10 min" });
    const fifteen = screen.getByRole("button", { name: "15 min" });
    const twentyFive = screen.getByRole("button", { name: "25 min" });
    expect(twentyFive.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(ten);
    expect(ten.getAttribute("aria-pressed")).toBe("true");
    expect(twentyFive.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(fifteen);
    expect(fifteen.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(screen.getByRole("button", { name: "Start session" }));
    expect(starts).toEqual([15]);
  });

  it("keeps the large 16-week activity heatmap and its legend", () => {
    renderToday();
    expect(ACTIVITY_DAYS).toBe(112);
    const cells = document.querySelectorAll(".activity-heatmap .activity-cell");
    expect(cells).toHaveLength(ACTIVITY_DAYS);
    expect(screen.getByText("Less")).toBeTruthy();
    expect(screen.getByText("More")).toBeTruthy();
    expect(screen.getByText("Today: 2 answers")).toBeTruthy();
    expect(
      screen.getByRole("img", { name: /answers in recent weeks/ }),
    ).toBeTruthy();
  });

  it("shows compact sync cues only when offline or work is pending", () => {
    renderToday({
      ...initialStatus(false),
      pendingOutbox: 3,
      persistence: "denied",
    });
    expect(screen.getByText("Offline. New answers stay on this device.")).toBeTruthy();
    expect(screen.getByText("3 answers waiting to sync.")).toBeTruthy();
  });

  it("shows explicit empty states before any learner activity", () => {
    renderToday(
      initialStatus(true),
      null,
      activitySummary([], null, Date.now()),
    );
    expect(
      screen.getByText("No recurring mistakes yet. Your first answers will surface them here."),
    ).toBeTruthy();
    expect(
      screen.getByText("No route focus yet. Your first session will create a note here."),
    ).toBeTruthy();
    expect(screen.getByText("No activity in recent weeks yet.")).toBeTruthy();
    expect(screen.getAllByText("Pre-A1 → A1").length).toBeGreaterThan(0);
    expect(screen.getByText("Your level grows after the first session.")).toBeTruthy();
  });
});
