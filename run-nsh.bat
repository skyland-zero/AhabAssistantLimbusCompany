@echo off
setlocal
cd /d "%~dp0"

if exist "%~dp0.venv\Scripts\python.exe" (
    set "AHAB_PYTHON=%~dp0.venv\Scripts\python.exe"
)

rem Faster Rust rebuilds: debug info and linker for the dev hosts.
call "%~dp0scripts\build-env.bat"

echo Starting AALC Native Shell App...
nsh run nsh-app %*

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    pause
)
