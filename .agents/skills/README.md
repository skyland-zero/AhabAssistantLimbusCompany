# 项目级 Agent Skills

这里是从 [longbridge/nsh](https://github.com/longbridge/nsh) 官方仓库同步过来的 Agent Skills（[agentskills.io 规范](https://agentskills.io/specification)），
随仓库提交，任何机器 clone 下来、支持 Agent Skills 的工具都能直接发现并加载。

| Skill | 用途 |
| --- | --- |
| `nsh/` | NSH 应用的构建、运行、测试与打包：Rust / TSX / 混合三模式、响应式、组件、样式、Tailwind、工具链，以及可运行的 app recipe |
| `nsh-design-guides/` | NSH 的规范性设计指南：层级、色彩与 token、间距、密度、组件选择、交互状态、浮层、动效、数据密集视图、界面文案，以及每次改 UI 都要走的「完成检查清单」 |

## 什么时候会被用上

写 `nsh-app/` 的 Rust 后端或 TSX 前端、改任何页面/样式/动效时。
混合模式（Rust 后端 + TSX 前端）先读 `nsh/references/mixed.md`，它讲的正是本项目采用的边界规则；
改界面前读 `nsh-design-guides/references/design-guides.md`。

## 来源与更新

```text
仓库：https://github.com/longbridge/nsh
路径：skills/
同步提交：e8d1100 text: Place inline views on their wrapped line in headless layout (#1787)
同步日期：2026-11-12
nsh CLI：0.5.0
```

更新方式（保持目录结构一致，只覆盖这两个 skill）：

```bash
git clone --depth 1 --filter=blob:none --sparse https://github.com/longbridge/nsh.git /tmp/nsh-src
cd /tmp/nsh-src && git sparse-checkout set skills
cd - && rm -rf .agents/skills/nsh .agents/skills/nsh-design-guides
cp -r /tmp/nsh-src/skills/nsh /tmp/nsh-src/skills/nsh-design-guides .agents/skills/
# 然后更新本文件里的「同步提交 / 同步日期」
```

本目录是上游内容的副本，**不要在这里手改**：需要改动先进上游，再同步回来；
项目自己的约定写在 `nsh-app/AGENTS.md` 与根 `AGENTS.md`。

## 说明

- 根 `.gitignore` 把 agent 目录整体忽略，但用 `!.agents/skills/` 放行了本目录；
  `.agents/` 下的其它内容（本地缓存、会话状态等）仍然不入库。
- 上游目前**没有提供 MCP server**，Agent Skills 与 `https://nsh.dev/llms.txt` 是它给 AI agent 的接入方式。
