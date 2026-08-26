import { useTranslation } from "react-i18next";

export function PlaceholderPage({
  pageKey,
}: {
  pageKey: "teams" | "themes" | "toolbox" | "resources";
}) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-8">
      <h1 className="text-xl font-semibold">{t(`pages.${pageKey}.title`)}</h1>
      <p className="text-sm text-muted-foreground">{t(`pages.${pageKey}.desc`)}</p>
      <p className="mt-4 rounded-md bg-muted px-3 py-1 text-xs text-muted-foreground">
        M3 里程碑实现
      </p>
    </div>
  );
}
