import { ChevronDown, ChevronUp, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

export interface PreviewTag {
  label: string;
  value?: string;
  highlight?: boolean;
}

interface FixedTaskCardProps {
  id: string;
  title: string;
  icon: LucideIcon;
  enabled: boolean;
  canDisable?: boolean;
  expanded: boolean;
  isExecuting?: boolean;
  previewTags?: PreviewTag[];
  onToggleEnabled?: (enabled: boolean) => void;
  onToggleExpanded?: () => void;
  children?: ReactNode;
}

export function FixedTaskCard({
  title,
  icon: Icon,
  enabled,
  canDisable = true,
  expanded,
  isExecuting = false,
  previewTags = [],
  onToggleEnabled,
  onToggleExpanded,
  children,
}: FixedTaskCardProps) {
  const hasOptions = Boolean(children);

  return (
    <Card
      className={cn(
        "group relative overflow-hidden py-0 gap-0 transition-all duration-200",
        isExecuting && "border-brand shadow-sm",
        !enabled && canDisable && "opacity-75 bg-muted/20",
      )}
    >
      {/* 运行中左侧指示线 */}
      {isExecuting && (
        <div className="absolute bottom-0 left-0 top-0 w-[3px] overflow-hidden">
          <div
            className="h-1/2 w-full bg-brand"
            style={{ animation: "sweep 1.2s linear infinite" }}
          />
        </div>
      )}

      {/* 卡片头部 */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: 卡片头部点击展开折叠 */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: 支持点击展开 */}
      <div
        className={cn("flex items-center gap-2.5 px-3 py-1.5 select-none")}
        onClick={hasOptions ? onToggleExpanded : undefined}
      >
        {/* 开关/复选框 */}
        {canDisable ? (
          /* biome-ignore lint/a11y/noStaticElementInteractions: 阻止冒泡 */
          <div onClick={(e) => e.stopPropagation()} onKeyDown={(e) => e.stopPropagation()}>
            <Switch
              checked={enabled}
              onCheckedChange={onToggleEnabled}
              className="data-[state=checked]:bg-brand"
            />
          </div>
        ) : (
          <div className="flex size-5 items-center justify-center rounded bg-muted text-[10px] text-muted-foreground font-mono">
            SET
          </div>
        )}

        {/* 标题与图标 */}
        <div className="flex items-center gap-2 min-w-0">
          <Icon
            className={cn(
              "size-4 shrink-0 transition-colors",
              isExecuting ? "text-brand" : "text-muted-foreground group-hover:text-foreground",
            )}
          />
          <span
            className={cn(
              "text-sm font-medium truncate",
              isExecuting && "text-brand font-semibold",
              !enabled && canDisable && "text-muted-foreground",
            )}
          >
            {title}
          </span>
        </div>

        {/* 折叠状态下的选项胶囊标签 (对齐 MXU) */}
        {!expanded && previewTags.length > 0 && (
          <div className="ml-auto hidden sm:flex items-center gap-1.5 overflow-hidden">
            {previewTags.map((tag) => (
              <Badge
                key={tag.label}
                variant={tag.highlight ? "default" : "secondary"}
                className={cn(
                  "h-5 px-1.5 text-[11px] font-normal",
                  tag.highlight && "bg-brand/10 text-brand",
                )}
              >
                <span>{tag.label}</span>
                {tag.value && <span className="font-semibold ml-0.5">{tag.value}</span>}
              </Badge>
            ))}
          </div>
        )}

        {/* 展开/收起箭头 */}
        {hasOptions && (
          <div className={cn("shrink-0 text-muted-foreground", !previewTags.length && "ml-auto")}>
            {expanded ? (
              <ChevronUp className="size-4 transition-transform" />
            ) : (
              <ChevronDown className="size-4 transition-transform" />
            )}
          </div>
        )}
      </div>

      {/* 展开后的详细配置内容 */}
      {hasOptions && expanded && (
        <CardContent className="animate-in fade-in slide-in-from-top-1 bg-muted/35 px-3 py-2.5 duration-150">
          {children}
        </CardContent>
      )}
    </Card>
  );
}
