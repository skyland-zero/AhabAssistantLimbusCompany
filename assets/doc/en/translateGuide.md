# Translation Guide

[简体中文](../zh/translateGuide.md) | **English**

## Documentation

1. Fork this repository.
2. Translate `README.md` or documents under `assets/doc/` while preserving Markdown formatting.
3. Submit a Pull Request.

## Native Shell interface translations

Native Shell catalogs live under `nsh-app/native/src/i18n/`, and TSX reads them
through `I18n.text()` and `I18n.feedback()`:

1. Select or add the Rust catalog for the target language (`zh_cn.rs` / `en_us.rs`).
2. Keep translation keys unchanged and edit only the localized text.
3. Preserve `{}` and other placeholders exactly.
4. Run formatting and Rust tests before submitting.

```powershell
cargo +1.99.1 fmt --manifest-path nsh-app/native/Cargo.toml
cargo +1.99.1 test --manifest-path nsh-app/native/Cargo.toml
$env:AHAB_BACKEND='mock'; nsh test nsh-app
```

Qt Linguist, `.ts`, `.qm`, and `lrelease` are no longer required.

## Images and OCR resources

When updating game-recognition resources, add images under `assets/images/` with
the expected directory structure and filenames. Verify them against the real
game window before submitting a Pull Request.
