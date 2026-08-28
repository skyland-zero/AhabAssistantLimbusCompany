# Translation Guide

[简体中文](../zh/translateGuide.md) | **English**

## Documentation

1. Fork this repository.
2. Translate `README.md` or documents under `assets/doc/` while preserving Markdown formatting.
3. Submit a Pull Request.

## GPUI interface translations

GPUI translation tables live under `gpui-app/src/i18n/`:

1. Select or add the Rust translation table for the target language.
2. Keep translation keys unchanged and edit only the localized text.
3. Preserve `{}` and other placeholders exactly.
4. Run formatting and Rust tests before submitting.

```powershell
cargo +nightly fmt --manifest-path gpui-app/Cargo.toml
cargo +nightly test --manifest-path gpui-app/Cargo.toml
```

Qt Linguist, `.ts`, `.qm`, and `lrelease` are no longer required.

## Images and OCR resources

When updating game-recognition resources, add images under `assets/images/` with
the expected directory structure and filenames. Verify them against the real
game window before submitting a Pull Request.
