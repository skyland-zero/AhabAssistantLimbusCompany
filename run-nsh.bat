@echo off
setlocal
cd /d "%~dp0"

if exist "%~dp0.venv\Scripts\python.exe" (
    set "AHAB_PYTHON=%~dp0.venv\Scripts\python.exe"
)

echo Starting AALC (Native Shell)...
nsh run nsh-app --target gpui %*

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    echo Is the nsh CLI installed? See nsh-app\README.md
    pause
)
