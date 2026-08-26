import { useTranslation } from "react-i18next";
import Markdown from "react-markdown";
import { PageHeader } from "@/components/common/PageHeader";
import { ScrollArea } from "@/components/ui/scroll-area";
import helpEn from "@/content/help-en.md?raw";
import helpZh from "@/content/help-zh.md?raw";
import { cn } from "@/lib/utils";

/** 从 markdown 源提取 ## 标题作为目录（与渲染顺序一致） */
function extractToc(source: string): string[] {
  return [...source.matchAll(/^##\s+(.+)$/gm)].map((m) => m[1]);
}

export function HelpPage() {
  const { t, i18n } = useTranslation();
  const source = i18n.language === "en-US" ? helpEn : helpZh;
  const toc = extractToc(source);

  const jumpTo = (index: number) => {
    document
      .getElementById(`help-h2-${index}`)
      ?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <div className="flex h-full min-h-0 flex-col">
      <PageHeader title={t("help.docTitle")} description={t("help.mockNotice")} />

      <div className="flex min-h-0 flex-1">
        {/* 左：目录 */}
        <nav className="w-52 shrink-0 border-r border-border p-3">
          <p className="px-2 pb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t("help.toc")}
          </p>
          <ul className="flex flex-col gap-0.5">
            {toc.map((title, i) => (
              <li key={title}>
                <button
                  type="button"
                  onClick={() => jumpTo(i)}
                  className={cn(
                    "w-full rounded-md px-2 py-1.5 text-left text-xs text-muted-foreground transition-colors",
                    "hover:bg-muted hover:text-foreground",
                  )}
                >
                  {title}
                </button>
              </li>
            ))}
          </ul>
        </nav>

        {/* 右：正文 */}
        <ScrollArea className="min-w-0 flex-1">
          <article className="mx-auto max-w-2xl px-8 py-6 text-sm leading-relaxed [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_code]:font-mono [&_code]:text-xs [&_h2]:mb-2 [&_h2]:mt-8 [&_h2]:border-b [&_h2]:border-border [&_h2]:pb-1 [&_h2]:text-base [&_h2]:font-semibold [&_h3]:mb-1.5 [&_h3]:mt-5 [&_h3]:text-sm [&_h3]:font-semibold [&_h1]:hidden [&_li]:my-0.5 [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5 [&_p]:my-2 [&_strong]:font-semibold [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5">
            <Markdown
              components={{
                h2: ({ children }) => {
                  const index = toc.indexOf(String(children));
                  return <h2 id={`help-h2-${index >= 0 ? index : undefined}`}>{children}</h2>;
                },
              }}
            >
              {source}
            </Markdown>
          </article>
        </ScrollArea>
      </div>
    </div>
  );
}
