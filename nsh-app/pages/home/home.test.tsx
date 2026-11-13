// The console's contract: the task queue, its parameters, and the run bar.
import { expect, open, test } from "nsh/test";

const DESKTOP = { width: 844, height: 704, platform: "windows" as const };

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

test("the statistics cards open their detail dialogs", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:stats-daily-details");
  await app.waitFor("每日刷本明细");
  await app.waitFor("暂无每日数据");
  await app.press("id:stats-details-close");
  await app.press("id:stats-last-mirror-details");
  await app.waitFor("镜牢完成记录");
  await app.waitFor("暂无镜牢完成记录");
  await app.press("id:stats-details-close");
});

test("the log card opens a full run-log dialog", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:log-details");
  await app.waitFor("运行日志明细");
  await app.waitFor("暂无日志");
  await app.press("id:log-details-close");
  await app.press("id:link-start");
  await app.waitFor("Mock 任务已开始");
  await app.press("id:log-details");
  await app.waitFor("运行日志明细");
  expect(app.texts()).toContain("Mock 任务已开始");
});

test("the window card cycles resolution and async input", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:window-resolution");
  await app.waitFor("分辨率 1440P");
  await app.press("id:window-resolution");
  await app.waitFor("分辨率 1080P");
  await app.press("id:window-async-input");
  await app.waitFor("异步输入 开");
});

test("the device picker lists a device once refreshed", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[刷新]");
  await app.press("label:选择设备");
  await app.waitFor("MuMu 模拟器");
});
