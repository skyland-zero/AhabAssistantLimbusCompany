@echo off
setlocal
cd /d "%~dp0"

set "AHAB_BACKEND=mock"

echo Starting Ahab GPUI App in Mock mode...
cargo +nightly run --manifest-path gpui-app/Cargo.toml %*

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    pause
)
