import { describe, expect, it } from "vitest";

import { applyFormat } from "./noteFormat";

describe("note formatting", () => {
  it("turns the caret line into a heading and back", () => {
    const text = "# Title\n\nArticles\nmore";
    const caret = text.indexOf("Articles") + 3;
    const heading = applyFormat(text, caret, caret, "h2");
    expect(heading.text).toBe("# Title\n\n## Articles\nmore");
    const back = applyFormat(heading.text, heading.start, heading.start, "h2");
    expect(back.text).toBe(text);
    expect(applyFormat("## Old", 0, 0, "h1").text).toBe("# Old");
  });

  it("wraps and unwraps a selection in bold or italic", () => {
    const bold = applyFormat("say a cat", 4, 9, "bold");
    expect(bold.text).toBe("say **a cat**");
    expect(bold.text.slice(bold.start, bold.end)).toBe("a cat");
    const plain = applyFormat(bold.text, bold.start - 2, bold.end + 2, "bold");
    expect(plain.text).toBe("say a cat");
    const italic = applyFormat("x", 1, 1, "italic");
    expect(italic).toEqual({ text: "x**", start: 2, end: 2 });
  });

  it("prefixes every selected line and toggles the prefix off", () => {
    const text = "one\ntwo\n\nthree";
    const list = applyFormat(text, 0, 7, "list");
    expect(list.text).toBe("- one\n- two\n\nthree");
    expect(applyFormat(list.text, list.start, list.end, "list").text).toBe(text);
    expect(applyFormat("a thought", 2, 2, "quote").text).toBe("> a thought");
  });

  it("inserts a callout block or wraps lines in one", () => {
    const empty = applyFormat("Intro", 5, 5, "callout");
    expect(empty.text).toBe("> [!note] \n> Intro");
    const wrapped = applyFormat("rule\nexample", 0, 12, "callout");
    expect(wrapped.text).toBe("> [!note]\n> rule\n> example");
  });

  it("wraps a wiki link and leaves the caret inside", () => {
    const empty = applyFormat("See ", 4, 4, "wiki");
    expect(empty).toEqual({ text: "See [[]]", start: 6, end: 6 });
    const named = applyFormat("See Articles", 4, 12, "wiki");
    expect(named.text).toBe("See [[Articles]]");
  });
});
