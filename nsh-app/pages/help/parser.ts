// A small Markdown reader for the bundled help documents.
//
// The client renders its own Markdown rather than an embedded web view, so the
// same subset is parsed here: headings (levels 1–3), paragraphs, bullet and
// ordered lists, fenced code, and inline strong, code and links. The parser is
// pure, so `cargo test`'s equivalents run as plain tests instead.

/** One block of a document. */
export type HelpBlock =
  | { kind: "heading"; level: number; text: string }
  | { kind: "paragraph"; text: string }
  | { kind: "bullet"; text: string }
  | { kind: "ordered"; text: string }
  | { kind: "code"; text: string };

/** One piece of a block's line. */
export type InlinePart =
  | { kind: "text"; value: string }
  | { kind: "strong"; value: string }
  | { kind: "code"; value: string }
  | { kind: "link"; label: string; url: string };

/** Splits a document into the blocks the page draws. */
export function parseHelp(source: string): HelpBlock[] {
  const blocks: HelpBlock[] = [];
  let paragraph: string[] = [];
  let code = false;
  let codeLines: string[] = [];

  const flushParagraph = () => {
    if (paragraph.length > 0) {
      blocks.push({ kind: "paragraph", text: paragraph.join(" ") });
      paragraph = [];
    }
  };
  const flushCode = () => {
    if (codeLines.length > 0) {
      blocks.push({ kind: "code", text: codeLines.join("\n") });
      codeLines = [];
    }
  };

  for (const line of source.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("```")) {
      if (code) flushCode();
      else flushParagraph();
      code = !code;
      continue;
    }
    if (code) {
      codeLines.push(line);
      continue;
    }
    if (trimmed === "") {
      flushParagraph();
      continue;
    }
    const heading = parseHeading(trimmed);
    if (heading) {
      flushParagraph();
      blocks.push({ kind: "heading", level: heading.level, text: inlineText(heading.text) });
      continue;
    }
    if (trimmed.startsWith("- ") || trimmed.startsWith("* ")) {
      flushParagraph();
      blocks.push({ kind: "bullet", text: trimmed.slice(2) });
      continue;
    }
    const ordered = parseOrderedItem(trimmed);
    if (ordered !== null) {
      flushParagraph();
      blocks.push({ kind: "ordered", text: ordered });
      continue;
    }
    paragraph.push(trimmed);
  }
  if (code) flushCode();
  flushParagraph();
  return blocks;
}

/** `## Title` → `{ level: 2, text: "Title" }`; only levels 1–3 count. */
export function parseHeading(line: string): { level: number; text: string } | null {
  let hashes = 0;
  while (line[hashes] === "#") hashes += 1;
  if (hashes < 1 || hashes > 3 || line[hashes] !== " ") return null;
  return { level: hashes, text: line.slice(hashes + 1).trim() };
}

/** `1. step` → `"1.  step"` (the client widened the gap); anything else `null`. */
export function parseOrderedItem(line: string): string | null {
  const split = line.indexOf(". ");
  if (split <= 0) return null;
  const number = line.slice(0, split);
  for (const character of number) {
    if (character < "0" || character > "9") return null;
  }
  return `${number}.  ${line.slice(split + 2)}`;
}

/** Splits a line into text, strong, code and link parts. */
export function parseInline(text: string): InlinePart[] {
  const parts: InlinePart[] = [];
  let remaining = text;
  while (remaining.length > 0) {
    const candidates = [
      remaining.indexOf("**"),
      remaining.indexOf("`"),
      remaining.indexOf("["),
    ].filter((index) => index >= 0);
    if (candidates.length === 0) {
      parts.push({ kind: "text", value: remaining });
      break;
    }
    const index = Math.min(...candidates);
    if (index > 0) {
      parts.push({ kind: "text", value: remaining.slice(0, index) });
      remaining = remaining.slice(index);
      continue;
    }
    if (remaining.startsWith("**")) {
      const end = remaining.indexOf("**", 2);
      if (end >= 0) {
        parts.push({ kind: "strong", value: remaining.slice(2, end) });
        remaining = remaining.slice(end + 2);
        continue;
      }
      parts.push({ kind: "text", value: "**" });
      remaining = remaining.slice(2);
      continue;
    }
    if (remaining.startsWith("`")) {
      const end = remaining.indexOf("`", 1);
      if (end >= 0) {
        parts.push({ kind: "code", value: remaining.slice(1, end) });
        remaining = remaining.slice(end + 1);
        continue;
      }
      parts.push({ kind: "text", value: "`" });
      remaining = remaining.slice(1);
      continue;
    }
    // A link: `[label](url)`; anything shorter stays literal text.
    const labelEnd = remaining.indexOf("](", 1);
    if (labelEnd >= 0) {
      const urlEnd = remaining.indexOf(")", labelEnd + 2);
      if (urlEnd >= 0) {
        parts.push({
          kind: "link",
          label: remaining.slice(1, labelEnd),
          url: remaining.slice(labelEnd + 2, urlEnd),
        });
        remaining = remaining.slice(urlEnd + 1);
        continue;
      }
    }
    parts.push({ kind: "text", value: "[" });
    remaining = remaining.slice(1);
  }
  return parts;
}

/** The plain text of a line, with its inline marks removed. */
export function inlineText(text: string): string {
  return parseInline(text)
    .map((part) => (part.kind === "link" ? part.label : part.value))
    .join("");
}

/** Which table-of-contents entry the reader is in: the last block index at or
 *  above the viewport's first visible block. */
export function activeToc(topIndex: number, tocIndices: number[]): number {
  let active = 0;
  for (let index = 0; index < tocIndices.length; index += 1) {
    if (tocIndices[index] <= topIndex) active = index;
  }
  return active;
}
