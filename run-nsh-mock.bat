@echo off
setlocal
cd /d "%~dp0"

set "AHAB_BACKEND=mock"

echo Starting AALC (Native Shell) in Mock mode...
nsh run nsh-app --target gpui %*

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    echo Is the nsh CLI installed? See nsh-app\README.md
    pause
)
