import type { Translation } from "./zh-CN";

const enUS: Translation = {
  app: {
    name: "Ahab Assistant · Limbus Company",
  },
  nav: {
    home: "Console",
    teams: "Teams",
    themes: "Themes",
    toolbox: "Toolbox",
    resources: "Resources",
    settings: "Settings",
    collapse: "Collapse sidebar",
    expand: "Expand sidebar",
  },
  titlebar: {
    minimize: "Minimize",
    maximize: "Maximize",
    unmaximize: "Restore",
    close: "Close",
  },
  home: {
    title: "Console",
    queueTitle: "Task Queue",
    addTask: "Add Task",
    screenshotTab: "Screenshot",
    logsTab: "Logs",
    emptyQueue: 'Queue is empty. Click "Add Task" to start',
    mockNotice: "M0 stage: UI shell only, backend not wired yet",
  },
  pages: {
    teams: {
      title: "Teams",
      desc: "Configure lineups for each stage (aligned with legacy team_setting_card)",
    },
    themes: {
      title: "Theme Packs",
      desc: "Manage installed UI theme packs (theme_pack_list.yaml)",
    },
    toolbox: {
      title: "Toolbox",
      desc: "Always-on-top, OCR test, notification test and more",
    },
    resources: {
      title: "Resources",
      desc: "Template & ONNX model sync status (resource_sync)",
    },
    settings: {
      title: "Settings",
      desc: "General / Appearance / Hotkeys / Notifications / Update / About",
    },
  },
  ipc: {
    connected: "Mock backend connected",
    methodNotFound: "Method not implemented",
  },
};

export default enUS;
