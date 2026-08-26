"use client";

import {
	CircleCheckIcon,
	InfoIcon,
	Loader2Icon,
	OctagonXIcon,
	TriangleAlertIcon,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Toaster as Sonner, type ToasterProps } from "sonner";
import { useAppStore } from "@/stores/appStore";
import { resolveSystemMode, watchSystemMode } from "@/themes";

const Toaster = ({ ...props }: ToasterProps) => {
	// 不依赖 next-themes：直接从 appStore 取主题模式并自行解析 system
	const themeMode = useAppStore((s) => s.themeMode);
	const [resolved, setResolved] = useState<"light" | "dark">(resolveSystemMode);

	useEffect(() => {
		if (themeMode !== "system") {
			setResolved(themeMode);
			return;
		}
		setResolved(resolveSystemMode());
		return watchSystemMode(() => setResolved(resolveSystemMode()));
	}, [themeMode]);

	return (
		<Sonner
			theme={resolved}
			className="toaster group"
			icons={{
				success: <CircleCheckIcon className="size-4" />,
				info: <InfoIcon className="size-4" />,
				warning: <TriangleAlertIcon className="size-4" />,
				error: <OctagonXIcon className="size-4" />,
				loading: <Loader2Icon className="size-4 animate-spin" />,
			}}
			style={
				{
					"--normal-bg": "var(--popover)",
					"--normal-text": "var(--popover-foreground)",
					"--normal-border": "var(--border)",
					"--border-radius": "var(--radius)",
				} as React.CSSProperties
			}
			{...props}
		/>
	);
};

export { Toaster };
