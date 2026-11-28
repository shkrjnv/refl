import type { ReactNode } from "react";

function inline(text: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*)/g);
  parts.forEach((part, index) => {
    if (part.startsWith("**") && part.endsWith("**") && part.length > 4) {
      nodes.push(<strong key={index}>{part.slice(2, -2)}</strong>);
    } else if (part.startsWith("*") && part.endsWith("*") && part.length > 2) {
      nodes.push(<em key={index}>{part.slice(1, -1)}</em>);
    } else if (part.length > 0) {
      nodes.push(part);
    }
  });
  return nodes;
}

export function RichText({ text }: { text: string }) {
  return <>{inline(text)}</>;
}
