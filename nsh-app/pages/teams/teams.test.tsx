// The teams page's contract: the filter counts, the formation slots, and the
// editor that writes a team back.
import { expect, open, test } from "nsh/test";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

test("the toolbar counts every purpose and the grid shows the saved teams", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[队伍管理]");
  for (const label of ["全部", "镜牢", "经验本", "通用", "编队 1 (震颤)", "编队 2 (烧伤)"]) {
    await app.waitFor(label);
  }
  expect(app.texts()).toContain("3");
  await app.waitFor("#04");
  await app.waitFor("未配置");
});

test("a filter narrows the grid to its purpose", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[队伍管理]");
  await app.press("id:team-filter-luxcavation");
  await app.waitFor("编队 2 (烧伤)");
  await app.waitFor("未配置");
  expect(app.texts()).not.toContain("编队 1 (震颤)");
});

test("the editor saves a renamed team", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[队伍管理]");
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.waitFor("队伍");
  // One ASCII key: the field is controlled, so this proves the draft round-trip.
  await app.type("id:team-name", "-");
  await app.press("id:save-team");
  await app.waitFor("编队 1 (震颤)-");
});

test("a new team takes the first free formation slot", async () => {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[队伍管理]");
  await app.press("id:new-team");
  await app.waitFor("队伍");
  await app.press("id:save-team");
  await app.waitFor("编队 4");
});
