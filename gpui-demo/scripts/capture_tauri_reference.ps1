[CmdletBinding()]
param(
    [string]$OutputDirectory = "",
    [ValidateSet("900x680", "800x560")]
    [string[]]$Sizes = @("900x680", "800x560"),
    [ValidateSet("zh-CN", "en-US")]
    [string[]]$Languages = @("zh-CN", "en-US"),
    [ValidateSet("light", "dark")]
    [string[]]$Themes = @("light", "dark"),
    [ValidateSet("home", "teams", "themes", "toolbox", "resources", "help", "settings")]
    [string[]]$Pages = @("home", "teams", "themes", "toolbox", "resources", "help", "settings")
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repo "artifacts/visual/reference-ui"
} else {
    $OutputDirectory = (Resolve-Path $OutputDirectory -ErrorAction SilentlyContinue)?.Path ?? (Join-Path (Get-Location) $OutputDirectory)
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$capture = (Resolve-Path (Join-Path $PSScriptRoot "../tools/capture_window.py")).Path
$log = Join-Path $OutputDirectory "tauri-dev.log"
$errLog = Join-Path $OutputDirectory "tauri-dev.log.err"
Remove-Item $log, $errLog -Force -ErrorAction SilentlyContinue

if (-not (Get-Command agent-browser -ErrorAction SilentlyContinue)) {
    throw "agent-browser is required; install it before running the Tauri capture"
}
if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    throw "python is required for native client resizing"
}

function Invoke-AgentBrowser {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & agent-browser --cdp 9222 @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "agent-browser failed: $($Arguments -join ' ')"
    }
}

function Wait-TauriTarget {
    $deadline = (Get-Date).AddSeconds(120)
    do {
        try {
            Invoke-WebRequest "http://localhost:9222/json/version" -UseBasicParsing -TimeoutSec 2 | Out-Null
            & agent-browser --cdp 9222 tab | Out-Null
            if ($LASTEXITCODE -eq 0) {
                return
            }
        } catch {
            # The WebView2 debugging endpoint appears only after Tauri creates
            # the window; keep polling while Rust/Vite finish starting.
        }
        Start-Sleep -Seconds 1
    } while ((Get-Date) -lt $deadline)
    throw "Tauri WebView2 CDP target did not become ready within 120 seconds"
}

function Resize-TauriClient {
    param([int]$Width, [int]$Height)
    & python $capture --logical-width $Width --logical-height $Height --resize-only --settle-ms 800
    if ($LASTEXITCODE -ne 0) {
        throw "failed to resize Tauri client to ${Width}x${Height}"
    }
}

$tauri = $null
try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
    $tauri = Start-Process -FilePath "pnpm.cmd" -ArgumentList "--dir", "ui", "tauri", "dev" -WorkingDirectory $repo -RedirectStandardOutput $log -RedirectStandardError $errLog -PassThru
    Wait-TauriTarget
    # Tauri's first launch compiles the Rust shell and creates the WebView2
    # profile. Wait after the target is visible so captures never contain the
    # startup/skeleton frame.
    Start-Sleep -Seconds 2

    foreach ($size in $Sizes) {
        $parts = $size -split "x"
        $logicalWidth = [int]$parts[0]
        $logicalHeight = [int]$parts[1]
        Resize-TauriClient $logicalWidth $logicalHeight
        foreach ($language in $Languages) {
            $languageShort = if ($language -eq "zh-CN") { "zh" } else { "en" }
            foreach ($theme in $Themes) {
                foreach ($page in $Pages) {
                    $state = @{
                        currentPage = $page
                        sidebarCollapsed = $false
                        rightPanelWidth = 280
                        rightPanelCollapsed = $false
                        themeMode = $theme
                        accentId = "crimson"
                        language = $language
                    } | ConvertTo-Json -Compress
                    $stateEnvelope = @{ state = (ConvertFrom-Json $state); version = 0 } | ConvertTo-Json -Compress
                    $bytes = [Text.Encoding]::UTF8.GetBytes($stateEnvelope)
                    $encoded = [Convert]::ToBase64String($bytes)
                    $script = "localStorage.setItem('ahab-ui-settings', atob('$encoded')); location.reload(); 'reload'"
                    $scriptBytes = [Text.Encoding]::UTF8.GetBytes($script)
                    $scriptEncoded = [Convert]::ToBase64String($scriptBytes)
                    Invoke-AgentBrowser eval -b $scriptEncoded | Out-Null
                    Invoke-AgentBrowser wait 900 | Out-Null
                    $output = Join-Path $OutputDirectory "tauri-$languageShort-$theme-$page-$size.png"
                    Invoke-AgentBrowser screenshot $output | Out-Null
                    Write-Host "captured $output"
                }
            }
        }
    }
} finally {
    Get-Process -Name "ui" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    if ($tauri -and (Get-Process -Id $tauri.Id -ErrorAction SilentlyContinue)) {
        & taskkill.exe /PID $tauri.Id /T /F 2>$null | Out-Null
    }
    Remove-Item Env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS -ErrorAction SilentlyContinue
}
