/** Markdown edits behind the notes toolbar and its shortcuts. */
export type FormatAction =
  | "h1"
  | "h2"
  | "h3"
  | "bold"
  | "italic"
  | "list"
  | "quote"
  | "callout"
  | "wiki";

export interface TextEdit {
  text: string;
  /** Selection after the edit. */
  start: number;
  end: number;
}

function lineBounds(text: string, start: number, end: number): { from: number; to: number } {
  const from = text.lastIndexOf("\n", Math.max(0, start - 1)) + 1;
  const newline = text.indexOf("\n", end > start && text[end - 1] === "\n" ? end - 1 : end);
  return { from, to: newline < 0 ? text.length : newline };
}

function replaceLines(
  text: string,
  start: number,
  end: number,
  map: (lines: string[]) => string[],
): TextEdit {
  const { from, to } = lineBounds(text, start, end);
  const lines = map(text.slice(from, to).split("\n"));
  const replaced = lines.join("\n");
  const next = text.slice(0, from) + replaced + text.slice(to);
  return { text: next, start: from, end: from + replaced.length };
}

function wrap(text: string, start: number, end: number, marker: string): TextEdit {
  const selected = text.slice(start, end);
  if (
    selected.length > 2 * marker.length &&
    selected.startsWith(marker) &&
    selected.endsWith(marker)
  ) {
    const inner = selected.slice(marker.length, -marker.length);
    return {
      text: text.slice(0, start) + inner + text.slice(end),
      start,
      end: start + inner.length,
    };
  }
  const next = text.slice(0, start) + marker + selected + marker + text.slice(end);
  return {
    text: next,
    start: start + marker.length,
    end: end + marker.length,
  };
}

function prefixLines(text: string, start: number, end: number, prefix: string): TextEdit {
  return replaceLines(text, start, end, (lines) => {
    const content = lines.filter((line) => line.trim());
    const all = content.length > 0 && content.every((line) => line.startsWith(prefix));
    return lines.map((line) => {
      if (!line.trim()) {
        return line;
      }
      return all ? line.slice(prefix.length) : `${prefix}${line}`;
    });
  });
}

/**
 * Applies a toolbar action to `text` with the selection `[start, end)`.
 * Headings, lists, quotes and callouts work on whole lines; bold, italic and
 * wiki links wrap the selection (or insert empty markers at the caret).
 */
export function applyFormat(
  text: string,
  start: number,
  end: number,
  action: FormatAction,
): TextEdit {
  const lo = Math.max(0, Math.min(start, end, text.length));
  const hi = Math.max(lo, Math.min(Math.max(start, end), text.length));
  switch (action) {
    case "h1":
    case "h2":
    case "h3": {
      const hashes = "#".repeat(Number(action.slice(1)));
      return replaceLines(text, lo, hi, (lines) =>
        lines.map((line) => {
          const bare = line.replace(/^#{1,6}\s+/, "");
          // Pressing the same level again turns the heading back into text.
          return line.startsWith(`${hashes} `) ? bare : `${hashes} ${bare}`;
        }),
      );
    }
    case "bold":
      return wrap(text, lo, hi, "**");
    case "italic":
      return wrap(text, lo, hi, "*");
    case "list":
      return prefixLines(text, lo, hi, "- ");
    case "quote":
      return prefixLines(text, lo, hi, "> ");
    case "callout": {
      if (lo === hi) {
        const { from } = lineBounds(text, lo, hi);
        const before = from > 0 && text[from - 1] !== "\n" ? "\n" : "";
        const block = `${before}> [!note] `;
        const next = text.slice(0, from) + block + "\n> " + text.slice(from);
        const caret = from + block.length;
        return { text: next, start: caret, end: caret };
      }
      return replaceLines(text, lo, hi, (lines) => [
        "> [!note]",
        ...lines.map((line) => `> ${line}`),
      ]);
    }
    case "wiki": {
      const selected = text.slice(lo, hi);
      const next = `${text.slice(0, lo)}[[${selected}]]${text.slice(hi)}`;
      return { text: next, start: lo + 2, end: lo + 2 + selected.length };
    }
  }
}
