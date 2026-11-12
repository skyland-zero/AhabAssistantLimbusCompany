// The Markdown subset the help page renders, and the directory's scroll rule.
import { expect, test } from "nsh/test";
import {
  activeToc,
  inlineText,
  parseHeading,
  parseHelp,
  parseInline,
  parseOrderedItem,
} from "./parser";

test("the parser extracts headings, lists and code without a web view", () => {
  const blocks = parseHelp("# Title\n\n## Section\n\n- **item**\n1. step\n\n```\ncode\n```");
  expect(blocks).toEqual([
    { kind: "heading", level: 1, text: "Title" },
    { kind: "heading", level: 2, text: "Section" },
    { kind: "bullet", text: "**item**" },
    { kind: "ordered", text: "1.  step" },
    { kind: "code", text: "code" },
  ]);
});

test("inline markdown keeps bold, code and links apart", () => {
  expect(parseInline("**bold** and [link](https://example.test) `code`")).toEqual([
    { kind: "strong", value: "bold" },
    { kind: "text", value: " and " },
    { kind: "link", label: "link", url: "https://example.test" },
    { kind: "text", value: " " },
    { kind: "code", value: "code" },
  ]);
  expect(inlineText("**bold** and [link](https://example.test) `code`")).toBe("bold and link code");
});

test("headings and ordered items follow the client's rules", () => {
  expect(parseHeading("### Details")).toEqual({ level: 3, text: "Details" });
  expect(parseHeading("#### Too deep")).toBe(null);
  expect(parseHeading("#NoSpace")).toBe(null);
  expect(parseOrderedItem("12. item")).toBe("12.  item");
  expect(parseOrderedItem("a. item")).toBe(null);
});

test("the directory follows the last heading at or above the viewport", () => {
  const toc = [0, 4, 9];
  expect(activeToc(0, toc)).toBe(0);
  expect(activeToc(7, toc)).toBe(1);
  expect(activeToc(99, toc)).toBe(2);
});
