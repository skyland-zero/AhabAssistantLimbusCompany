@echo off
setlocal
cd /d "%~dp0"

set "AHAB_BACKEND=mock"

rem Faster Rust rebuilds: debug info and linker for the dev hosts.
call "%~dp0scripts\build-env.bat"

echo Starting AALC Native Shell App in Mock mode...
nsh run nsh-app %*

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    pause
)
