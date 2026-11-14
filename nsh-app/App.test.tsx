// The shell's smoke tests: the title bar lists every destination, a press
// switches the page, the settings utility opens Settings, and the appearance
// toggle cycles and reports what it shows.
import { expect, open, test } from "nsh/test";

/** The window the desktop client runs in (`nsh.json` 820×680); the headless
 *  default is a phone, where the title bar would be clipped. */
const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the title bar lists every destination", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.waitFor("AALC");
  for (const label of ["主控台", "队伍管理", "镜牢主题包", "工具箱"]) {
    await app.waitFor(`role:button[${label}]`);
  }
});

test("pressing a destination switches the page", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.waitFor("日常任务");
  await app.press("role:button[工具箱]");
  await app.waitFor("自动战斗");
  await app.press("role:button[主控台]");
  await app.waitFor("日常任务");
});

test("the settings utility opens the settings page", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.waitFor("快速导航");
  await app.waitFor("主题模式");
});

test("the appearance toggle cycles through the three appearances", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[外观：跟随系统]");
  await app.waitFor("role:button[外观：浅色]");
  await app.press("role:button[外观：浅色]");
  await app.waitFor("role:button[外观：深色]");
});

// The page switch is a cross-dissolve: the page that leaves fades out while the
// one that arrives fades in, on the same curve, so at every frame the pair adds
// up to one full page — the old page's text never reads through the new page's
// empty areas, and the window's backdrop never shows.
//
// The fade is driven by a manual clock so the mid-fade frame is a fact rather
// than a race with the test host: on the system clock the assertions below would
// depend on how fast the machine renders the 200 ms entrance.
test("a page switch fades the old page out of the tree", async () => {
  const app = await open("./App.tsx", { ...DESKTOP, clock: {} });
  const clock = app.clock!;
  await app.waitFor("日常任务");
  await app.press("role:button[工具箱]");
  await app.waitFor("自动战斗");
  // Mid-fade both pages are up: the console has not left the tree yet, it is
  // only fading. This is what keeps the window's backdrop from showing.
  expect(app.texts()).toContain("日常任务");
  // Run past the fade (200 ms) plus the layer hold (60 ms): the leaving page is
  // dropped and the arriving one is the only one left.
  await clock.runFor(400);
  expect(app.texts()).not.toContain("日常任务");
});

test("the language switch updates the title bar and page copy", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[设置]");
  await app.waitFor("外观");
  await app.press("role:button[English]");
  await app.waitFor("Teams");
  // Help now lives behind Settings → About; its own language switch follows.
  await app.press("role:button[关于]");
  await app.wait(250);
  await app.press("role:button[查看帮助]");
  await app.waitFor("Contents");
  expect(app.texts()).not.toContain("队伍管理");
  // Put the interface back: the shell keeps one language for the whole file, so
  // leaving it English here is what used to fail the tests after this one. The
  // help page's own switch also offers both languages, so the settings page's
  // segmented control is `nth=0` — until its layer is dropped (the page fade
  // plus its hold), so wait for the switch to finish before pressing.
  await app.wait(400);
  await app.press("role:button[简体中文] >> nth=0");
  await app.waitFor("队伍管理");
});

test("every destination keeps the shared window chrome", async () => {
  const app = await open("./App.tsx", DESKTOP);
  await app.press("role:button[工具箱]");
  await app.waitFor("自动战斗");
  expect(app.texts()).toContain("AALC");
});
