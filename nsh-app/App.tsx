// The entry: the window shell, the skin's background layer, and the page host.
// Page logic lives in `pages/`, the shell in `shell/`, shared state in
// `store.ts`. The root fills the whole window: `nsh.json` asks for a frameless
// transparent window, so the app paints every pixel of it, corners included.
import { mount, View } from "nsh";
import { Runtime } from "aalc/native";
import { PageHost } from "./pages/PageHost";
import { DesktopIntegration } from "./shell/DesktopIntegration";
import { SkinBackdrop } from "./shell/SkinBackdrop";
import { TitleBar } from "./shell/TitleBar";
import { currentPage, initTheme } from "./store";

function App() {
  // Once, before the first frame: the composed theme replaces `nsh.json`'s
  // boot theme, which only carries the default skin, and the stored appearance
  // replaces the system's.
  initTheme();
  Runtime.enableTransparency();
  return (
    <View class="relative flex-1 overflow-hidden bg-background">
      <DesktopIntegration />
      <SkinBackdrop />
      <TitleBar />
      <View class="min-h-0 flex-1">
        <PageHost page={currentPage()} />
      </View>
    </View>
  );
}

mount(App);
