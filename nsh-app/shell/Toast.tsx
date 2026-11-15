// The app's transient messages: one toast stack at the window's bottom-right.
//
// `nsh/base` owns the lifecycle and the stack's motion; this file supplies the
// look, keeps the visible entries in a signal, and exposes `notify()` to the
// state modules so a result can be shown without adding another footer line.
import { Overlay, Text, effect, safeArea, signal, token } from "nsh";
import { Toast, ToastManager, ToastStack } from "nsh/base";
import type { ToastEntry } from "nsh/base";
import { Icon } from "../components/Icon";
import type { IconName } from "../components/Icon";

export type ToastTone = "success" | "info" | "warning" | "danger";

type ToastMessage = { title: string; tone: ToastTone };

const manager = new ToastManager();
const shown = signal<ToastEntry[]>([]);
const expanded = signal(false);
let nextId = 0;

function sync(): void {
  shown.set(manager.visible(3));
}

/** Shows a short result. Ordinary setting toggles stay silent; only a result
 *  the control cannot express (an update check, a failed action) belongs here. */
export function notify(
  title: string,
  options: { tone?: ToastTone; timeout?: number; id?: string } = {},
): void {
  if (!title.trim()) return;
  manager.push(
    options.id ?? `toast-${(nextId += 1)}`,
    { title, tone: options.tone ?? "info" } satisfies ToastMessage,
    { timeout: options.timeout ?? 4000 },
  );
  sync();
}

const GLYPH: Record<ToastTone, IconName> = {
  success: "check",
  info: "sparkles",
  warning: "help",
  danger: "close",
};

const TINT: Record<ToastTone, "success" | "brand" | "warning" | "danger"> = {
  success: "success",
  info: "brand",
  warning: "warning",
  danger: "danger",
};

/** Mounted once in `App.tsx`, above the pages. */
export function Toaster() {
  effect(() => {
    if (shown().length === 0) return;
    const timer = setInterval(() => {
      if (manager.advance(expanded()).changed) sync();
    }, 50);
    return () => clearInterval(timer);
  });

  const dismissNewest = () => {
    const entries = shown();
    for (let index = entries.length - 1; index >= 0; index -= 1) {
      if (entries[index].status !== "ending") {
        manager.dismiss(entries[index].id);
        sync();
        return;
      }
    }
  };

  return (
    <Overlay
      class="pointer-events-none inset-x-0 bottom-0 items-end p-4"
      style={{ paddingBottom: safeArea().bottom + 16 }}
    >
      <ToastStack
        placement="bottomRight"
        class="pointer-events-auto w-80"
        onExpandedChange={({ expanded: next }) => expanded.set(next)}
        onCancel={dismissNewest}
      >
        <Each of={shown()} key={(entry) => entry.id}>
          {(entry) => {
            const message = () => (entry().value as ToastMessage);
            return (
              <Toast
                transitionStatus={entry().status}
                accessibilityLabel={message().title}
                onDismissRequest={() => {
                  manager.dismiss(entry().id);
                  sync();
                }}
                class="w-full flex-row items-start gap-2 rounded-(--radius-lg) border border-input/60 bg-popover px-4 py-3 shadow-lg transition-all duration-200 motion-reduce:transition-none"
                startingClass="opacity-0 translate-y-2"
                endingClass="opacity-0 scale-95"
              >
                <Icon
                  name={GLYPH[message().tone]}
                  size={14}
                  color={token(TINT[message().tone])}
                  class="mt-0.5"
                />
                <Text class="min-w-0 flex-1 text-xs leading-5 text-popover-foreground">
                  {message().title}
                </Text>
              </Toast>
            );
          }}
        </Each>
      </ToastStack>
    </Overlay>
  );
}
