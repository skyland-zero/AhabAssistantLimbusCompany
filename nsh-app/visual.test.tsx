// Real-window screenshots for the migration's visual comparison.
//
// Headless `nsh test` has no window to draw, so the captures are skipped there.
// Run it on a graphical target to write the baselines under `artifacts/nsh/`
// (the directory must exist):
//
//   nsh test nsh-app/visual.test.tsx --target windows
//
// Every shot is the client's 820×680 card, which `nsh.json` opens as an
// 844×704 window (the frame's 12pt band around it), so a shot lines up with
// `nsh-app/docs/reference/gpui/`.
import { open, test } from "nsh/test";

const DESKTOP = { width: 844, height: 704, platform: "windows" as const };
const OUT = "artifacts/nsh";

const PAGES: { label: string; marker: string; shot: string }[] = [
  { label: "主控台", marker: "Python 后端", shot: "home" },
  { label: "队伍管理", marker: "全部", shot: "teams" },
  { label: "镜牢主题包", marker: "总权重：", shot: "theme-packs" },
  { label: "工具箱", marker: "自动战斗", shot: "toolbox" },
  { label: "资源中心", marker: "检查更新", shot: "resources" },
  { label: "帮助", marker: "目录", shot: "help" },
  { label: "设置", marker: "外观", shot: "settings" },
];

async function shot(app: Awaited<ReturnType<typeof open>>, name: string): Promise<void> {
  try {
    await app.screenshot(`${OUT}/${name}.png`);
    await app.wait(250);
  } catch (error) {
    // A headless run draws nothing. Only swallow that case; a real capture
    // failure (a missing directory, a closed window) still fails the test.
    if (!String(error).includes("screenshot")) throw error;
  }
}

function pagesTest(name: string, scheme: "light" | "dark"): void {
  test(name, async () => {
    const app = await open("./App.tsx", { ...DESKTOP, colorScheme: scheme });
    for (const page of PAGES) {
      await app.press(`role:button[${page.label}]`);
      await app.waitFor(page.marker);
      await shot(app, `${scheme}-${page.shot}`);
    }
  });
}

function editorTest(name: string, scheme: "light" | "dark"): void {
  test(name, async () => {
    const app = await open("./App.tsx", { ...DESKTOP, colorScheme: scheme });
    await app.press("role:button[队伍管理]");
    // The page swap commits after the press returns; without this the editor
    // button is still missing when the suite runs under load.
    await app.waitFor("编队 1 (震颤)");
    await app.press("role:button[编辑 编队 1 (震颤)]");
    await app.waitFor("编辑队伍");
    for (const tab of ["basic", "shop", "combat", "starlight", "advanced"] as const) {
      await app.press(`id:team-tab-${tab}`);
      await shot(app, `${scheme}-teams-${tab}`);
    }
  });
}

function detailsTest(name: string, scheme: "light" | "dark"): void {
  test(name, async () => {
    const app = await open("./App.tsx", { ...DESKTOP, colorScheme: scheme });
    await app.press("id:stats-daily-details");
    await app.waitFor("每日刷本明细");
    await shot(app, `${scheme}-stats-daily`);
    await app.press("id:stats-details-close");
    await app.press("id:stats-last-mirror-details");
    await app.waitFor("镜牢完成记录");
    await shot(app, `${scheme}-stats-mirror`);
    await app.press("id:stats-details-close");
    await app.press("id:link-start");
    await app.waitFor("Mock 任务已开始");
    await app.press("id:log-details");
    await app.waitFor("运行日志明细");
    await shot(app, `${scheme}-log-details`);
  });
}

pagesTest("visual baseline: pages in light", "light");
pagesTest("visual baseline: pages in dark", "dark");
editorTest("visual baseline: team editor in light", "light");
editorTest("visual baseline: team editor in dark", "dark");
detailsTest("visual baseline: console details in light", "light");
detailsTest("visual baseline: console details in dark", "dark");
