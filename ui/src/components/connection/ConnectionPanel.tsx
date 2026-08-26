import { Circle, Loader2, Monitor, RefreshCw, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { getIpc } from "@/services/ipc/client";
import type { ConnectionStatus, DeviceInfo, DeviceStatusPayload } from "@/services/ipc/types";

const statusBadge: Record<ConnectionStatus, { key: string; className: string }> = {
  disconnected: { key: "connection.disconnected", className: "bg-muted text-muted-foreground" },
  connecting: {
    key: "connection.connecting",
    className: "bg-warning-light text-warning dark:bg-warning/20",
  },
  connected: {
    key: "connection.connected",
    className: "bg-success-light text-success dark:bg-success/20",
  },
};

/** 设备连接面板（对齐 MXU ConnectionPanel）：选择游戏窗口后自动连接 */
export function ConnectionPanel() {
  const { t } = useTranslation();
  const [devices, setDevices] = useState<DeviceInfo[]>([]);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>("disconnected");
  const [scanning, setScanning] = useState(false);

  useEffect(() => {
    let unsub: (() => void) | undefined;
    void (async () => {
      const ipc = await getIpc();
      setDevices(await ipc.request<DeviceInfo[]>("device.list"));
      unsub = ipc.on("device.status", (payload) => {
        const { deviceId: id, status: s } = payload as DeviceStatusPayload;
        setDeviceId(id);
        setStatus(s);
      });
    })();
    return () => void unsub?.();
  }, []);

  const rescan = async () => {
    setScanning(true);
    setDevices(await (await getIpc()).request<DeviceInfo[]>("device.list"));
    // 保持扫描态一小会儿，让转圈可见
    setTimeout(() => setScanning(false), 400);
  };

  const selectDevice = async (id: string) => {
    await (await getIpc()).request("device.connect", { id });
  };

  const disconnect = async () => {
    await (await getIpc()).request("device.disconnect");
  };

  const badge = statusBadge[status];

  return (
    <div className="flex shrink-0 flex-col overflow-hidden rounded-lg border border-border bg-card">
      <div className="flex h-9 shrink-0 items-center justify-between border-b border-border px-3">
        <span className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
          <Monitor className="size-3.5" /> {t("connection.title")}
        </span>
        <Badge className={cn("gap-1 px-1.5 text-[10px]", badge.className)}>
          <Circle
            className={cn("size-1.5 fill-current", status === "connecting" && "animate-pulse")}
          />
          {t(badge.key)}
        </Badge>
      </div>
      <div className="flex items-center gap-2 p-3">
        <Select
          value={deviceId ?? undefined}
          disabled={status === "connecting"}
          onValueChange={(v) => void selectDevice(v)}
        >
          <SelectTrigger size="sm" className="min-w-0 flex-1 text-xs">
            <SelectValue placeholder={t("connection.selectPlaceholder")} />
          </SelectTrigger>
          <SelectContent>
            {devices.map((d) => (
              <SelectItem key={d.id} value={d.id}>
                {d.name}
                {d.detail && (
                  <span className="ml-1 font-mono text-[10px] text-muted-foreground">
                    {d.detail}
                  </span>
                )}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="ghost"
          size="icon"
          className="size-7 shrink-0"
          disabled={status === "connecting"}
          onClick={() => void rescan()}
          aria-label={t("connection.refresh")}
          title={t("connection.refresh")}
        >
          <RefreshCw className={cn("size-3.5", scanning && "animate-spin")} />
        </Button>
        {status === "connected" && (
          <Button
            variant="ghost"
            size="icon"
            className="size-7 shrink-0"
            onClick={() => void disconnect()}
            aria-label={t("connection.disconnect")}
            title={t("connection.disconnect")}
          >
            <X className="size-3.5" />
          </Button>
        )}
        {status === "connecting" && (
          <Loader2 className="size-3.5 shrink-0 animate-spin text-brand" />
        )}
      </div>
    </div>
  );
}
