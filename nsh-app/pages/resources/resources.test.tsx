// The resources page's contract: the groups and their versions, the update
// check, and a sync that ends with a settled time.
import { open, test } from "nsh/test";
import { expect } from "nsh/test";
import { formatSyncTime } from "./Resources";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the page lists the resource groups and their versions", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[资源中心]");
  await app.waitFor("模板资源");
  await app.waitFor("ONNX 模型");
  expect(app.texts()).toContain("v2025.06.1");
  expect(app.texts()).toContain("已是最新");
  expect(app.texts()).toContain("从未同步");
});

test("checking for updates says what it did", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[资源中心]");
  await app.press("role:button[检查更新]");
  await app.waitFor("发现资源更新");
});

test("a sync reports progress and settles", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[资源中心]");
  await app.press("role:button[立即同步]");
  await app.waitFor("资源同步完成");
});

test("a zero timestamp is never reported as a real sync", () => {
  expect(formatSyncTime(null)).toBe("从未同步");
  expect(formatSyncTime(0)).toBe("从未同步");
  expect(formatSyncTime(1700000000)).toBe("时间戳 1700000000");
});
