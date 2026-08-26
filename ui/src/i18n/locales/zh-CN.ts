const zhCN = {
  app: {
    name: "Ahab Assistant · Limbus Company",
  },
  nav: {
    home: "主控台",
    teams: "队伍",
    themes: "主题包",
    toolbox: "工具箱",
    resources: "资源中心",
    settings: "设置",
    collapse: "收起导航",
    expand: "展开导航",
  },
  titlebar: {
    minimize: "最小化",
    maximize: "最大化",
    unmaximize: "还原",
    close: "关闭",
  },
  home: {
    title: "主控台",
    queueTitle: "任务队列",
    addTask: "添加任务",
    screenshotTab: "截图",
    logsTab: "日志",
    emptyQueue: "队列为空，点击「添加任务」开始",
    mockNotice: "M0 阶段：界面骨架，后端尚未接入",
  },
  pages: {
    teams: {
      title: "队伍管理",
      desc: "配置各副本使用的编队（对齐旧版 team_setting_card）",
    },
    themes: {
      title: "主题包",
      desc: "管理已安装的界面主题包（theme_pack_list.yaml）",
    },
    toolbox: {
      title: "工具箱",
      desc: "窗口置顶、OCR 测试、通知测试等实用工具",
    },
    resources: {
      title: "资源中心",
      desc: "模板资源与 ONNX 模型的同步状态（resource_sync）",
    },
    settings: {
      title: "设置",
      desc: "通用 / 外观 / 热键 / 通知 / 更新 / 关于",
    },
  },
  ipc: {
    connected: "Mock 后端已连接",
    methodNotFound: "方法未实现",
  },
};

export default zhCN;
export type Translation = typeof zhCN;
