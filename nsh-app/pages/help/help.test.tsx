// The help page's contract: the directory lists the document's sections and a
// press reaches one. Help is reached from Settings → About, not the title bar.
import { open, test } from "nsh/test";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

/** Opens the app on the help page: Settings, then the About card's button. */
async function openHelp() {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.waitFor("快速导航");
  await app.press("role:button[关于]");
  await app.wait(250);
  await app.press("role:button[查看帮助]");
  await app.waitFor("目录");
  return app;
}

test("the directory lists the bundled document's sections", async () => {
  const app = await openHelp();
  for (const title of ["目录", "快速上手", "任务类型", "完成后动作", "工具箱", "常见问题"]) {
    await app.waitFor(title);
  }
});

test("the document renders its lists and inline code", async () => {
  const app = await openHelp();
  await app.waitFor("1.  在「队伍」页面创建编队，选择人格与饰品体系");
  // Inline code is its own run, so it is its own text.
  await app.waitFor("shutdown /s /t 60");
});

test("pressing a directory entry reaches that section", async () => {
  const app = await openHelp();
  await app.press("role:button[常见问题]");
  await app.waitFor("窗口识别失败？");
});

test("switching the document's language reloads it", async () => {
  const app = await openHelp();
  await app.press("role:button[English]");
  await app.waitFor("Quick Start");
});
