// The teams page's contract: the forty slot map, the mirror rotation, the
// configured team cards, the preset picker and the editor round-trip.
import { expect, open, test } from "nsh/test";

const DESKTOP = { width: 820, height: 680, platform: "windows" as const };

// The page swap commits after `press` returns; under the full suite the grid is
// still empty when the next press runs, so every test waits for the slot map.
async function openTeams() {
  const app = await open("../../App.tsx", DESKTOP);
  await app.press("role:button[队伍管理]");
  await app.waitFor("编队位（游戏内 1–40）");
  return app;
}

test("the toolbar counts every purpose and the slot map shows the saved teams", async () => {
  const app = await openTeams();
  for (const label of ["全部", "镜牢", "资源本", "通用", "#01 编队 1 (震颤)", "#02 编队 2 (烧伤)"]) {
    await app.waitFor(label);
  }
  expect(app.texts()).toContain("3");
  await app.waitFor("#04");
  await app.waitFor("AALC.nsh 未配置");
  await app.waitFor("镜牢轮换");
});

test("a filter narrows the saved-team list to its purpose", async () => {
  const app = await openTeams();
  await app.press("id:team-filter-luxcavation");
  await app.waitFor("#02 编队 2 (烧伤)");
  expect(app.texts()).toContain("#02 编队 2 (烧伤)");
  expect(app.texts()).not.toContain("#01 编队 1 (震颤)");
});

test("the editor saves a renamed team", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.waitFor("编辑编队 #01");
  // One ASCII key: the field is controlled, so this proves the draft round-trip.
  await app.type("id:team-name", "-");
  await app.press("id:save-team");
  await app.waitFor("#01 编队 1 (震颤)-");
});

test("editing inside the dialog does not rebuild it", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.waitFor("编辑编队 #01");
  // A rebuild between keys drops the input node, so only the first character
  // would land; this guards the dialog's `open` binding against the draft.
  await app.type("id:team-name", "abc");
  expect(app.value("id:team-name")).toBe("编队 1 (震颤)abc");
  await app.press("id:team-sinner-faust");
  expect(app.value("id:team-name")).toBe("编队 1 (震颤)abc");
});

test("a new team takes the first free formation slot", async () => {
  const app = await openTeams();
  await app.press("id:new-team");
  await app.waitFor("新建编队 #04");
  await app.press("id:save-team");
  await app.waitFor("#04 编队 4");
});

test("clicking an empty slot creates that exact number", async () => {
  const app = await openTeams();
  await app.press("id:slot-8");
  await app.waitFor("新建编队 #08");
  await app.press("id:save-team");
  await app.waitFor("#08 编队 8");
});

test("the preset picker applies a template into a free slot", async () => {
  const app = await openTeams();
  await app.press("id:preset-new");
  await app.waitFor("从预设新建");
  await app.press("id:preset-spiderweb_exp_luxcavation");
  await app.press("role:button[使用并编辑]");
  await app.waitFor("新建编队 #04");
  await app.press("id:save-team");
  await app.waitFor("#04 蜘蛛巢经验本（8人）");
});

test("the editor exposes all five mirror sections and the starlight cost", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  for (const tab of ["基础编成", "商店与合成", "二体系与战斗", "开局星光", "观测与高级"]) {
    await app.waitFor(tab);
  }
  await app.press("id:team-tab-shop");
  await app.waitFor("舍弃饰品体系");
  await app.waitFor("镜牢路线");
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

test("a resource team only offers the basic section", async () => {
  const app = await openTeams();
  await app.press("id:team-filter-luxcavation");
  await app.press("role:button[编辑 编队 2 (烧伤)]");
  await app.waitFor("编辑编队 #02");
  expect(app.texts()).toContain("基础编成");
  expect(app.texts()).not.toContain("商店与合成");
  expect(app.texts().some((text) => text.includes("资源本优先使用编队码"))).toBe(true);
});

test("a general team is manual-only", async () => {
  const app = await openTeams();
  await app.press("id:team-filter-general");
  await app.press("role:button[编辑 编队 3 (呼吸)]");
  await app.waitFor("编辑编队 #03");
  expect(app.texts().some((text) => text.includes("通用队伍仅手动使用"))).toBe(true);
  expect(app.texts()).not.toContain("加入镜牢轮换");
  expect(app.texts()).not.toContain("商店与合成");
});

test("deleting a team asks for confirmation first", async () => {
  const app = await openTeams();
  await app.press("role:button[删除 编队 3 (呼吸)]");
  await app.waitFor("删除编队 #03？");
  await app.press("role:button[取消]");
  await app.waitFor("#03 编队 3 (呼吸)");
  await app.press("role:button[删除 编队 3 (呼吸)]");
  await app.waitFor("删除编队 #03？");
  await app.press("role:button[删除]");
  await app.waitFor("队伍已删除");
  expect(app.texts()).not.toContain("#03 编队 3 (呼吸)");
});

test("the editor shows the formation source and the route", async () => {
  const app = await openTeams();
  await app.press("role:button[编辑 编队 1 (震颤)]");
  await app.waitFor("编队来源");
  await app.press("role:button[编队码]");
  expect(app.texts().some((text) => text.includes("编队码为空"))).toBe(true);
  await app.press("id:team-tab-shop");
  await app.waitFor("镜牢路线");
  await app.press("label:镜牢路线");
  await app.press("text:蜘蛛巢普牢速刷路线");
  await app.press("id:save-team");
  await app.waitFor("蜘蛛巢普牢速刷路线");
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

test("the selection-mode switch explains which game name/order it uses", async () => {
  const app = await openTeams();
  await app.press("label:选队方式");
  await app.press("text:按序号");
  await app.waitFor("已改为按序号选队：游戏内编队顺序需要保持 1–40");
});
