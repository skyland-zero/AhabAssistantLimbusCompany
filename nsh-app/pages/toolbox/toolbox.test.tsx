// The toolbox's contract: every tool is listed with its action, and running one
// shows that it is running.
import { open, test } from "nsh/test";

const DESKTOP = { width: 844, height: 704, platform: "windows" as const };

test("the toolbox lists every tool and the sidecar note", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[工具箱]");
  for (const title of ["自动战斗", "体力换饼", "辅助截图", "分辨率修改"]) {
    await app.waitFor(title);
  }
  await app.waitFor("工具请求通过 Python sidecar 执行");
});

test("running a tool switches it to stop and says so", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[工具箱]");
  await app.press("id:tool-action-infiniteBattle");
  await app.waitFor("工具已启动");
  await app.waitFor("运行中");
  await app.press("id:tool-action-infiniteBattle");
  await app.waitFor("工具已停止");
});

test("the resolution tool offers both commands", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[工具箱]");
  await app.press("role:button[修改 1080P]");
  await app.waitFor("已修改分辨率为 1080P (240 DPI)");
  await app.press("role:button[还原默认]");
  await app.waitFor("已还原设备分辨率与 DPI");
});
