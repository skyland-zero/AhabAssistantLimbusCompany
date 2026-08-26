import type { ReactNode } from "react";

/** 页面标题头：标题 + 描述 + 右侧动作区（children） */
export function PageHeader({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children?: ReactNode;
}) {
  return (
    <div className="flex shrink-0 items-center justify-between gap-4 px-4 py-3">
      <div className="min-w-0">
        <h1 className="truncate text-lg font-semibold">{title}</h1>
        {description && (
          <p className="mt-0.5 truncate text-xs text-muted-foreground">{description}</p>
        )}
      </div>
      {children && <div className="flex shrink-0 items-center gap-2">{children}</div>}
    </div>
  );
}
