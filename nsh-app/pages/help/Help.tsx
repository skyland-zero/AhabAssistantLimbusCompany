// The help page: a table of contents at 208pt and the bundled document beside
// it, rendered from the Markdown the app ships.
//
// The document is fetched once per language from the bundled asset and parsed
// with `parser.ts`; the directory lists its level-2 headings and follows the
// reader's scroll, and pressing one scrolls to that heading.
import type { ScrollRef } from "nsh";
import { Scroll, Text, View, computed, resource, signal } from "nsh";
import { Link } from "nsh/base";
import helpEn from "../../assets/content/help-en.md?url";
import helpZh from "../../assets/content/help-zh.md?url";
import { Button } from "../../components/Button";
import { Card, CardBody } from "../../components/Card";
import { Reveal } from "../../components/Reveal";
import { currentLanguage, setLanguage, t } from "../../settings";
import type { Language } from "../../settings";
import { activeToc, parseHelp, parseInline } from "./parser";
import type { InlinePart } from "./parser";
import type { HelpBlock } from "./parser";

/** How far past a heading still counts as "that section" (12pt). */
const SNAP = 12;

export function Help() {
  let scroll: ScrollRef | undefined;
  const language = () => currentLanguage();
  const document = resource(language, async (lang, { signal: abort }) => {
    const response = await fetch(lang === "zhCn" ? helpZh : helpEn, { signal: abort });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    return await response.text();
  });

  // The document's single level-1 heading is not shown, as the client hid it.
  const blocks = computed<HelpBlock[]>(() =>
    parseHelp(document() ?? "").filter(
      (block) => !(block.kind === "heading" && block.level === 1),
    ),
  );
  // The directory lists the level-2 headings and the block each one starts at.
  const toc = computed(() =>
    blocks()
      .map((block, index) => ({ block, index }))
      .filter((entry) => entry.block.kind === "heading" && entry.block.level === 2)
      .map((entry) => ({
        index: entry.index,
        title: entry.block.kind === "heading" ? entry.block.text : "",
      })),
  );

  const active = signal(0);
  const offsets = signal<Record<number, number>>({});
  const record = (index: number, y: number) =>
    offsets.update((current) => ({ ...current, [index]: y }));

  const jumpTo = (tocIndex: number) => {
    active.set(tocIndex);
    const blockIndex = toc()[tocIndex]?.index;
    if (blockIndex === undefined) return;
    scroll?.scrollTo({ y: Math.max(0, (offsets()[blockIndex] ?? 0) - SNAP), animated: true });
  };

  const follow = (event: { offset: { y: number } }) => {
    const y = event.offset.y + SNAP;
    let top = 0;
    for (const [index, blockTop] of Object.entries(offsets())) {
      if (blockTop <= y) top = Math.max(top, Number(index));
    }
    const next = activeToc(top, toc().map((entry) => entry.index));
    if (next !== active()) active.set(next);
  };

  return (
    <View class="size-full min-h-0 min-w-0 flex-row gap-2 p-2.5">
      <Card
        title={t("contents")}
        class="h-full w-52 flex-none"
        actions={
          <View class="flex-row items-center gap-1">
            <Each of={LANGUAGES}>
              {(entry) => (
                <Button
                  label={entry().label}
                  variant={language() === entry().value ? "secondary" : "ghost"}
                  selected={language() === entry().value}
                  size="xs"
                  class="px-2"
                  onPress={() => {
                    setLanguage(entry().value);
                    active.set(0);
                    scroll?.scrollTo({ y: 0 });
                  }}
                />
              )}
            </Each>
          </View>
        }
      >
        <CardBody class="gap-0.5">
          <If when={document.state() === "pending" || document.state() === "unresolved"}>
            <Text class="text-xs text-muted-foreground">加载中…</Text>
          </If>
          <If when={document.error()}>
            <Text class="text-xs text-destructive">无法加载帮助文档</Text>
          </If>
          <Each of={toc()}>
            {(entry, tocIndex) => (
              <Reveal index={tocIndex()} class="w-full">
                <Button
                  label={entry().title}
                  variant={active() === tocIndex() ? "secondary" : "ghost"}
                  selected={active() === tocIndex()}
                  class="w-full"
                  onPress={() => jumpTo(tocIndex())}
                />
              </Reveal>
            )}
          </Each>
        </CardBody>
      </Card>

      <Scroll ref={scroll} class="min-h-0 min-w-0 flex-1" onScroll={(e) => follow(e)}>
        <Card class="mx-auto w-full max-w-168 px-8 py-6">
          <View class="flex-col">
            <Each of={blocks()}>
              {(block, index) => <DocumentBlock block={block()} onOffset={record} index={index()} />}
            </Each>
          </View>
        </Card>
      </Scroll>
    </View>
  );
}

/** The two documents the app ships. */
const LANGUAGES: { value: Language; label: string }[] = [
  { value: "zhCn", label: "简体中文" },
  { value: "enUs", label: "English" },
];

/** One block, reporting where it starts so the directory can follow the reader. */
function DocumentBlock(props: {
  block: HelpBlock;
  index: number;
  onOffset: (index: number, y: number) => void;
}) {
  return (
    <View onLayout={({ bounds }) => props.onOffset(props.index, bounds.origin.y)}>
      <BlockBody block={props.block} />
    </View>
  );
}

/** One document block, in the reading styles the client used. */
function BlockBody(props: { block: HelpBlock }) {
  const block = props.block;
  if (block.kind === "heading") {
    // Level 2 opens a section and carries a rule; level 3 is a sub-heading.
    return block.level === 2 ? (
      <View class="mt-7 border-b border-border pb-1">
        <Text class="text-base font-semibold text-foreground">{block.text}</Text>
      </View>
    ) : (
      <View class="mt-4">
        <Text class="text-sm font-semibold text-foreground">{block.text}</Text>
      </View>
    );
  }
  if (block.kind === "code") {
    return (
      <View class="my-2 w-full rounded-(--radius) bg-popover p-3">
        <Text class="font-mono text-xs text-foreground">{block.text}</Text>
      </View>
    );
  }
  if (block.kind === "bullet") {
    return (
      <View class="my-1 flex-row items-start gap-2 ps-5">
        <Text class="flex-none text-sm text-foreground">•</Text>
        <Inline text={block.text} />
      </View>
    );
  }
  if (block.kind === "ordered") {
    return (
      <View class="my-1 ps-5">
        <Inline text={block.text} />
      </View>
    );
  }
  return (
    <View class="my-1.5">
      <Inline text={block.text} />
    </View>
  );
}

/** A line's runs: plain text, strong, inline code and links. */
function Inline(props: { text: string }) {
  return (
    <View class="flex-1 flex-row flex-wrap items-center">
      <Each of={parseInline(props.text)}>
        {(part) => <InlineRun part={part()} />}
      </Each>
    </View>
  );
}

function InlineRun(props: { part: InlinePart }) {
  if (props.part.kind === "strong") {
    return <Text class="text-sm font-semibold text-foreground">{props.part.value}</Text>;
  }
  if (props.part.kind === "code") {
    return (
      <Text class="rounded-sm bg-card px-1 font-mono text-xs text-foreground">
        {props.part.value}
      </Text>
    );
  }
  if (props.part.kind === "link") {
    // A link leaves the app for a resource, so it keeps the link role rather
    // than becoming a quiet button.
    return (
      <Link
        href={props.part.url}
        openWith="system"
        class="focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
      >
        <Text class="text-sm text-brand underline">{props.part.label}</Text>
      </Link>
    );
  }
  return <Text class="text-sm leading-6 text-foreground">{props.part.value}</Text>;
}
