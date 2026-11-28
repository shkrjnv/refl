// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { NoteMarkdown } from "./NoteMarkdown";

afterEach(cleanup);

describe("note Markdown", () => {
  it("renders headings, lists, callouts, and a linked highlight", () => {
    const { container } = render(
      <NoteMarkdown
        body={
          "# Title\n\nIntro with **bold** and `code`.\n\n## Steps\n\n- one\n- two\n\n> [!highlight] A quote\n> The cat sleeps.\n"
        }
      />,
    );
    expect(screen.getByRole("heading", { name: "Title" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Steps" })).toBeTruthy();
    expect(screen.getByText("bold").tagName).toBe("STRONG");
    expect(screen.getByText("code").tagName).toBe("CODE");
    expect(container.querySelectorAll("li")).toHaveLength(2);
    const callout = container.querySelector(".note-callout-highlight");
    expect(callout?.textContent).toContain("A quote");
    expect(callout?.textContent).toContain("The cat sleeps.");
  });

  it("makes wiki links clickable without navigating away", () => {
    const onWikiLink = vi.fn();
    render(
      <NoteMarkdown
        body="See [[note:n1|the other note]] and [[cat]]."
        onWikiLink={onWikiLink}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "the other note" }));
    expect(onWikiLink).toHaveBeenCalledWith("note:n1", "the other note");
    fireEvent.click(screen.getByRole("button", { name: "cat" }));
    expect(onWikiLink).toHaveBeenCalledWith("cat", "cat");
  });
});
