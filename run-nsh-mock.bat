@echo off
setlocal
cd /d "%~dp0"

set "AHAB_BACKEND=mock"

echo Starting AALC Native Shell App in Mock mode...
nsh run nsh-app %*

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    pause
)
