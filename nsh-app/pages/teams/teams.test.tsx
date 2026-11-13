// The teams page's contract: the filter counts, the formation slots, and the
// editor that writes a team back.
import { expect, open, test } from "nsh/test";

const DESKTOP = { width: 844, height: 704, platform: "windows" as const };

// The page swap commits after `press` returns; under the full suite the grid is
// still empty when the next press runs, so every test waits for the first card.
async function openTeams() {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[队伍管理]");
  await app.waitFor("编队 1 (震颤)");
  return app;
}

test("the toolbar counts every purpose and the grid shows the saved teams", async () => {
  const app = await openTeams();
  for (const label of ["全部", "镜牢", "经验本", "通用", "编队 1 (震颤)", "编队 2 (烧伤)"]) {
    await app.waitFor(label);
  }
  expect(app.texts()).toContain("3");
  await app.waitFor("#04");
  await app.waitFor("未配置");
});

test("a filter narrows the grid to its purpose", async () => {
  const app = await openTeams();
  await app.press("id:team-filter-luxcavation");
  await app.waitFor("编队 2 (烧伤)");
  await app.waitFor("额外编队");
  expect(app.texts()).not.toContain("编队 1 (震颤)");
});

test("the editor saves a renamed team", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.waitFor("编辑队伍");
  // One ASCII key: the field is controlled, so this proves the draft round-trip.
  await app.type("id:team-name", "-");
  await app.press("id:save-team");
  await app.waitFor("编队 1 (震颤)-");
});

test("a new team takes the first free formation slot", async () => {
  const app = await openTeams();
  await app.press("id:new-team");
  await app.waitFor("新建队伍");
  await app.press("id:save-team");
  await app.waitFor("编队 4");
});

test("the editor exposes all five mirror sections and the starlight cost", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  for (const tab of ["基础编成", "商店与合成", "二体系与战斗", "开局星光", "观测与高级"]) {
    await app.waitFor(tab);
  }
  await app.press("id:team-tab-shop");
  await app.waitFor("舍弃饰品体系");
  await app.press("id:team-tab-starlight");
  await app.waitFor("总消耗 80 点");
  await app.press("id:team-tab-advanced");
  await app.waitFor("编队统计数据");
});

test("the shop tab toggles a discarded system and the starlight quick set", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.press("id:team-tab-shop");
  // Sinking is already discarded in the fixture; one press restores it.
  await app.press("role:button[沉沦]");
  await app.press("id:team-tab-starlight");
  await app.press("role:button[全部 2+]");
  await app.waitFor("总消耗 620 点");
});

test("an experience-dungeon team only offers the basic section", async () => {
  const app = await openTeams();
  await app.press("id:team-filter-luxcavation");
  await app.press("role:button[编辑 编队 2 (烧伤)]");
  await app.waitFor("编辑队伍");
  expect(app.texts()).toContain("基础编成");
  expect(app.texts()).not.toContain("商店与合成");
});

test("copying the editor puts the contract's team JSON on the clipboard", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.press("role:button[复制 JSON]");
  await app.wait(100);
  expect(app.clipboard()).toContain('"name": "编队 1 (震颤)"');
  expect(app.clipboard()).toContain('"opening_bonus"');
});

test("the advanced tab refreshes and clears the team's history", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.press("id:team-tab-advanced");
  await app.press("role:button[刷新数据]");
  await app.waitFor("困难镜牢次数");
  expect(app.texts()).toContain("6");
  await app.press("role:button[清除历史统计数据]");
  await app.waitFor("将删除这支队伍的全部镜牢统计数据，且无法恢复。");
  await app.press("role:button[清除]");
  await app.waitFor("队伍统计已清空");
});
