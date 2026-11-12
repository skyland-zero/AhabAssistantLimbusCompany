// The console's contract: the task queue, its parameters, and the run bar.
import { expect, open, test } from "nsh/test";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the console shows the backend, this week's summary and the task queue", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  for (const text of [
    "Python 后端",
    "Mock 模式",
    "周期统计（今日/本周）",
    "上次镜牢",
    "窗口设置",
    "日常任务",
    "领取奖励",
    "狂气换体",
    "自动镜牢",
    "亚哈共鸣",
    "设备连接",
    "实时画面",
    "运行日志",
  ]) {
    await app.waitFor(text);
  }
});

test("a task parameter keeps the value it is given", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("label:经验本");
  await app.press("role:option[×2]");
  await app.waitFor("×2");
});

test("select all and clear act on the queue", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:clear-tasks");
  await app.waitFor("已清空任务");
  await app.press("id:select-all");
  await app.waitFor("已全选任务");
});

test("starting the queue says what it did", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:link-start");
  await app.waitFor("已提交启动请求");
  await app.waitFor("Mock 任务已开始");
  expect(app.texts()).toContain("F10");
});

test("the device picker lists a device once refreshed", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[刷新]");
  await app.press("label:选择设备");
  await app.waitFor("MuMu 模拟器");
});
