// The entry: the window's frame, the skin's background layer, the window shell,
// and the page host. Page logic lives in `pages/`, the shell in `shell/`,
// shared state in `store.ts`.
import { mount, View } from "nsh";
import { PageHost } from "./pages/PageHost";
import { DesktopIntegration } from "./shell/DesktopIntegration";
import { SkinBackdrop } from "./shell/SkinBackdrop";
import { TitleBar } from "./shell/TitleBar";
import { WindowFrame } from "./shell/WindowFrame";
import { currentPage, initTheme } from "./store";

function App() {
  // Once, before the first frame: the composed theme replaces `nsh.json`'s
  // boot theme, which only carries the default skin, and the stored appearance
  // replaces the system's.
  initTheme();
  return (
    <View class="relative flex-1">
      <WindowFrame>
        <DesktopIntegration />
        <SkinBackdrop />
        <TitleBar />
        <View class="min-h-0 flex-1">
          <PageHost page={currentPage()} />
        </View>
      </WindowFrame>
    </View>
  );
}

mount(App);
