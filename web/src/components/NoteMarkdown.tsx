import type { ReactNode } from "react";

import { headingId } from "../prototype/notes";

export interface NoteMarkdownProps {
  body: string;
  onWikiLink?: (target: string, label: string) => void;
  className?: string;
  omitFirstHeading?: boolean;
}

const INLINE =
  /\[\[([^\]|]+)(?:\|([^\]]+))?\]\]|\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`|\[([^\]]+)\]\(([^)]+)\)/g;

function inline(text: string, onWikiLink?: (target: string, label: string) => void): ReactNode[] {
  const nodes: ReactNode[] = [];
  let cursor = 0;
  for (const match of text.matchAll(INLINE)) {
    const index = match.index ?? 0;
    if (index > cursor) {
      nodes.push(text.slice(cursor, index));
    }
    const [whole, wikiTarget, wikiLabel, bold, italic, code, linkLabel, href] = match;
    if (wikiTarget) {
      const label = wikiLabel?.trim() || wikiTarget.trim();
      nodes.push(
        <button
          key={`${index}-wiki`}
          type="button"
          className="wiki-link"
          onClick={() => onWikiLink?.(wikiTarget.trim(), label)}
        >
          {label}
        </button>,
      );
    } else if (bold) {
      nodes.push(<strong key={`${index}-bold`}>{bold}</strong>);
    } else if (italic) {
      nodes.push(<em key={`${index}-em`}>{italic}</em>);
    } else if (code) {
      nodes.push(
        <code key={`${index}-code`} className="note-inline-code">
          {code}
        </code>,
      );
    } else if (linkLabel && href) {
      const safe = /^(https?:|mailto:|\/)/i.test(href);
      nodes.push(
        safe ? (
          <a
            key={`${index}-link`}
            href={href}
            target="_blank"
            rel="noreferrer"
            className="note-external-link"
          >
            {linkLabel}
          </a>
        ) : (
          <span key={`${index}-link`}>{linkLabel}</span>
        ),
      );
    } else {
      nodes.push(whole);
    }
    cursor = index + whole.length;
  }
  if (cursor < text.length) {
    nodes.push(text.slice(cursor));
  }
  return nodes;
}

function isBlockStart(line: string): boolean {
  return (
    /^#{1,3}\s+/.test(line) ||
    /^\s*([-*+]|\d+\.)\s+/.test(line) ||
    line.trim().startsWith(">") ||
    line.trim().startsWith("```")
  );
}

function renderParagraphs(
  lines: string[],
  onWikiLink: ((target: string, label: string) => void) | undefined,
  keyPrefix: string,
): ReactNode[] {
  const out: ReactNode[] = [];
  let paragraph: string[] = [];
  let list: { text: string; ordered: boolean }[] = [];
  const flushParagraph = (index: number) => {
    if (paragraph.length === 0) {
      return;
    }
    out.push(
      <p key={`${keyPrefix}-p-${index}`}>
        {inline(paragraph.join(" "), onWikiLink)}
      </p>,
    );
    paragraph = [];
  };
  const flushList = (index: number) => {
    if (list.length === 0) {
      return;
    }
    const ordered = list[0]?.ordered === true;
    const items = list.map((entry, itemIndex) => (
      <li key={`${keyPrefix}-li-${index}-${itemIndex}`}>
        {inline(entry.text, onWikiLink)}
      </li>
    ));
    out.push(
      ordered ? (
        <ol key={`${keyPrefix}-ol-${index}`}>{items}</ol>
      ) : (
        <ul key={`${keyPrefix}-ul-${index}`}>{items}</ul>
      ),
    );
    list = [];
  };
  lines.forEach((line, index) => {
    const trimmed = line.trim();
    const unordered = /^[-*+]\s+(.+)$/.exec(trimmed);
    const ordered = /^\d+\.\s+(.+)$/.exec(trimmed);
    if (unordered || ordered) {
      flushParagraph(index);
      list.push({ text: (unordered?.[1] ?? ordered?.[1] ?? ""), ordered: Boolean(ordered) });
      return;
    }
    flushList(index);
    if (!trimmed) {
      flushParagraph(index);
      return;
    }
    paragraph.push(trimmed);
  });
  flushParagraph(lines.length);
  flushList(lines.length);
  return out;
}

/** A safe Markdown reader with wiki links, callouts, and collapsible highlights. */
export function NoteMarkdown({
  body,
  onWikiLink,
  className = "",
  omitFirstHeading = false,
}: NoteMarkdownProps) {
  let omittedHeading = false;
  const lines = body.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index] ?? "";
    const trimmed = line.trim();
    if (!trimmed) {
      index += 1;
      continue;
    }
    if (trimmed.startsWith("```")) {
      const code: string[] = [];
      index += 1;
      while (index < lines.length && !(lines[index] ?? "").trim().startsWith("```")) {
        code.push(lines[index] ?? "");
        index += 1;
      }
      index += 1;
      blocks.push(
        <pre key={`code-${index}`} className="note-code-block">
          <code>{code.join("\n")}</code>
        </pre>,
      );
      continue;
    }
    const heading = /^(#{1,3})\s+(.+)$/.exec(trimmed);
    if (heading) {
      const level = (heading[1] ?? "#").length;
      const text = heading[2] ?? "";
      const id = headingId(index, text);
      if (omitFirstHeading && level === 1 && !omittedHeading) {
        omittedHeading = true;
        blocks.push(
          <span key={`h-anchor-${index}`} id={id} className="note-heading-anchor" />,
        );
        index += 1;
        continue;
      }
      const children = inline(text, onWikiLink);
      if (level === 1) {
        blocks.push(<h2 key={`h-${index}`} id={id}>{children}</h2>);
      } else if (level === 2) {
        blocks.push(<h3 key={`h-${index}`} id={id}>{children}</h3>);
      } else {
        blocks.push(<h4 key={`h-${index}`} id={id}>{children}</h4>);
      }
      index += 1;
      continue;
    }
    if (trimmed.startsWith(">")) {
      const quote: string[] = [];
      while (index < lines.length && (lines[index] ?? "").trim().startsWith(">")) {
        quote.push((lines[index] ?? "").replace(/^\s*>\s?/, ""));
        index += 1;
      }
      const first = quote[0]?.trim() ?? "";
      const callout = /^\[!([a-z-]+)\]\s*(.*)$/i.exec(first);
      if (callout) {
        const kind = (callout[1] ?? "note").toLocaleLowerCase();
        const title = callout[2]?.trim() || kind;
        blocks.push(
          <details
            key={`callout-${index}`}
            className={`note-callout note-callout-${kind}`}
            open
          >
            <summary>{title}</summary>
            <div className="note-callout-body">
              {renderParagraphs(quote.slice(1), onWikiLink, `callout-${index}`)}
            </div>
          </details>,
        );
      } else {
        blocks.push(
          <blockquote key={`quote-${index}`} className="note-quote">
            {renderParagraphs(quote, onWikiLink, `quote-${index}`)}
          </blockquote>,
        );
      }
      continue;
    }
    if (/^\s*([-*+]|\d+\.)\s+/.test(line) || !isBlockStart(line)) {
      let end = index + 1;
      while (end < lines.length) {
        const next = lines[end] ?? "";
        if (!next.trim() || isBlockStart(next)) {
          break;
        }
        end += 1;
      }
      blocks.push(
        <div key={`paras-${index}`} className="note-markdown-group">
          {renderParagraphs(lines.slice(index, end), onWikiLink, `paras-${index}`)}
        </div>,
      );
      index = end;
      continue;
    }
    index += 1;
  }
  return <div className={`note-markdown ${className}`.trim()}>{blocks}</div>;
}
