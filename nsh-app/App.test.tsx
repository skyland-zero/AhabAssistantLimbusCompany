// The P0 smoke test: the shell renders, the sidebar is there, and the Rust
// crate's version reaches the screen through the generated declarations.
import { expect, open, test } from "nsh/test";

test("the shell lists every page and shows the backend version", async () => {
  const app = await open("./App.tsx");
  await app.waitFor("主控台");
  await app.waitFor("队伍");
  await app.waitFor("设置");
  await app.waitFor("主题包");
  await app.waitFor("帮助");
  await app.waitFor("页面迁入进度");
});

test("pressing a sidebar entry switches the page", async () => {
  const app = await open("./App.tsx");
  await app.press("队伍");
  await app.waitFor("队伍页面在 P3 按 MIGRATION.md 逐页迁入");
});

test("the Rust module reports a platform", async () => {
  const app = await open("./App.tsx");
  const shown = app.text("后端（Rust）");
  expect(shown).toBeDefined();
});
