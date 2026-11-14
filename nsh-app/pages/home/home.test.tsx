// The console's contract: the task queue, its parameters, and the run bar.
import { expect, open, test } from "nsh/test";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the console shows the backend, this week's summary and the task queue", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  for (const text of [
    "未启动 Python 后端",
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
  await app.press("id:task-toggle-daily");
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

test("the top card follows the running queue", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:link-start");
  await app.waitFor("Mock 任务已开始");
  // The card leaves the totals behind and states the run instead.
  await app.waitFor("周期统计（今日/本周）", { state: "gone" });
  await app.waitFor("运行中");
});

test("starting the queue says what it did", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:link-start");
  await app.waitFor("已提交启动请求");
  await app.waitFor("Mock 任务已开始");
  expect(app.texts()).toContain("F10");
});

test("the run bar turns Link Start into stop while the queue runs", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:link-start");
  await app.waitFor("Mock 任务已开始");
  await app.waitFor("停止");
  await app.press("id:link-start");
  await app.waitFor("Link Start!");
});

test("the statistics card opens the statistics page", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:stats-details");
  await app.waitFor("最近镜牢用时");
  await app.waitFor("每日次数");
  await app.press("role:button[主控台]");
  await app.waitFor("日常任务");
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

test("the window card expands to its settings", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:window-toggle");
  await app.waitFor("窗口分辨率");
  await app.press("label:窗口分辨率");
  await app.press("role:option[2560 × 1440]");
  await app.waitFor("分辨率 1440P");
  await app.press("label:异步 PostMessage 输入");
  await app.waitFor("异步输入 开");
  await app.press("label:窗口位置");
  await app.press("role:option[靠左对齐]");
  await app.press("label:截图间隔");
  await app.press("role:option[0.2s]");
  await app.press("label:减少误触");
  expect(app.texts()).toContain("0.2s");
  await app.press("id:window-toggle");
  expect(app.texts()).not.toContain("窗口分辨率");
});

test("the after-run actions save from the run bar", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("id:after-completion");
  await app.waitFor("结束后操作");
  await app.press("label:退出游戏");
  await app.press("label:电源动作");
  await app.press("role:option[锁屏]");
  await app.press("id:after-completion-save");
  await app.waitFor("退出游戏 · 锁屏");
  expect(app.texts()).toContain("默认");
  await app.press("id:after-completion");
  await app.press("label:退出游戏");
  await app.press("id:after-completion-once");
  await app.waitFor("锁屏");
  expect(app.texts()).toContain("本次");
});

test("the device picker lists a device once refreshed", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("label:刷新设备列表");
  // The refresh runs off the engine thread; wait for it to land before
  // opening the picker, which snapshots its options when it opens.
  await app.waitFor("已刷新设备列表");
  await app.press("label:选择设备");
  await app.waitFor("MuMu 模拟器");
});

test("choosing a device connects off the engine thread", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("label:刷新设备列表");
  await app.waitFor("已刷新设备列表");
  await app.press("label:选择设备");
  await app.press("role:option[MuMu 模拟器]");
  await app.waitFor("设备已连接");
  await app.waitFor("已连接");
  // The picker states the chosen device instead of falling back to its hint.
  expect(app.texts()).toContain("MuMu 模拟器");
});
