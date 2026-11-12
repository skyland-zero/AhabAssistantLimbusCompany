# 翻译指南

**简体中文** | [English](../en/translateGuide.md)

## 文档翻译

1. Fork 本仓库。
2. 翻译 `README.md` 或 `assets/doc/` 下的文档，并保持 Markdown 格式。
3. 提交修改并发起 Pull Request。

## Native Shell 界面翻译

Native Shell 的目录位于 `nsh-app/native/src/i18n/`，TSX 通过 `I18n.text()` 与
`I18n.feedback()` 读取：

1. 选择或新增对应语言的 Rust 目录（`zh_cn.rs` / `en_us.rs`）。
2. 保持翻译键不变，只修改对应语言的文本。
3. 如果文本包含 `{}` 等占位符，必须原样保留。
4. 运行 Rust 格式检查和测试，确认所有页面都能加载该语言。

```powershell
cargo +1.99.1 fmt --manifest-path nsh-app/native/Cargo.toml
cargo +1.99.1 test --manifest-path nsh-app/native/Cargo.toml
$env:AHAB_BACKEND='mock'; nsh test nsh-app
```

不再需要 Qt Linguist、`.ts`、`.qm` 或 `lrelease`。

## 图片和 OCR 资源

需要更新游戏识别资源时，按 `assets/images/` 中的目录和文件名添加图片，
并在真实游戏窗口中验证识别结果后提交 Pull Request。
