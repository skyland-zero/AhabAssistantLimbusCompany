[CmdletBinding()]
param(
    [string]$OutputDirectory = "",
    [string]$Executable = "",
    [ValidateSet("900x680", "800x560")]
    [string[]]$Sizes = @("900x680", "800x560"),
    [ValidateSet("zh-CN", "en-US")]
    [string[]]$Languages = @("zh-CN", "en-US"),
    [ValidateSet("light", "dark")]
    [string[]]$Themes = @("light", "dark"),
    [ValidateSet("home", "teams", "themes", "toolbox", "resources", "help", "settings")]
    [string[]]$Pages = @("home", "teams", "themes", "toolbox", "resources", "help", "settings"),
    [string[]]$States = @()
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repo "artifacts/visual/gpui"
} else {
    $OutputDirectory = (Resolve-Path $OutputDirectory -ErrorAction SilentlyContinue)?.Path ?? (Join-Path (Get-Location) $OutputDirectory)
}
if ([string]::IsNullOrWhiteSpace($Executable)) {
    $Executable = Join-Path $repo "gpui-app/target/release/ahab-gpui-app.exe"
}
$Executable = (Resolve-Path $Executable).Path
$capture = (Resolve-Path (Join-Path $PSScriptRoot "../tools/capture_window.py")).Path
$profile = Join-Path $OutputDirectory ".profile"
$settingsDirectory = Join-Path $profile "AhabAssistant"
New-Item -ItemType Directory -Force -Path $OutputDirectory, $settingsDirectory | Out-Null

$statePages = @{
    "home-expanded" = "home"
    "home-select" = "home"
    "home-running" = "home"
    "home-paused" = "home"
    "home-after-completion" = "home"
    "teams-editor" = "teams"
    "teams-shop-editor" = "teams"
    "teams-combat-editor" = "teams"
    "teams-starlight-editor" = "teams"
    "teams-advanced-editor" = "teams"
    "teams-delete" = "teams"
    "teams-select" = "teams"
    "settings-hotkey" = "settings"
    "settings-select" = "settings"
    "settings-latest" = "settings"
    "toolbox-running" = "toolbox"
    "resources-syncing" = "resources"
    "help-scrolled" = "help"
}
$States = @($States | ForEach-Object { $_ -split "," } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$unknownStates = @($States | Where-Object { -not $statePages.ContainsKey($_) })
if ($unknownStates.Count -gt 0) {
    throw "unknown visual state(s): $($unknownStates -join ', ')"
}

$cases = @()
if ($States.Count -gt 0) {
    foreach ($state in $States) {
        $cases += [pscustomobject]@{ Page = $statePages[$state]; State = $state; Suffix = $state }
    }
} else {
    foreach ($page in $Pages) {
        $cases += [pscustomobject]@{ Page = $page; State = ""; Suffix = $page }
    }
}

if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    throw "python is required for physical-pixel window capture"
}

function Stop-GpuiProcess {
    Get-Process -Name "ahab-gpui-app" -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 300
}

$settingsPath = Join-Path $settingsDirectory "settings.json"
$settings = @{
    version = 1
    sidebarCollapsed = $false
    rightPanelWidth = 280
    rightPanelCollapsed = $false
    themeMode = "light"
    accentId = "crimson"
    language = "zh-CN"
} | ConvertTo-Json
$settings | Set-Content -Encoding utf8 $settingsPath

try {
    Stop-GpuiProcess
    foreach ($size in $Sizes) {
        $parts = $size -split "x"
        $logicalWidth = [int]$parts[0]
        $logicalHeight = [int]$parts[1]
        foreach ($language in $Languages) {
            $languageShort = if ($language -eq "zh-CN") { "zh" } else { "en" }
            foreach ($theme in $Themes) {
                foreach ($case in $cases) {
                    $env:APPDATA = $profile
                    $env:AHAB_VISUAL_THEME = $theme
                    $env:AHAB_VISUAL_LANGUAGE = $language
                    $env:AHAB_VISUAL_ACCENT = "crimson"
                    $env:AHAB_VISUAL_PAGE = $case.Page
                    if ([string]::IsNullOrWhiteSpace($case.State)) {
                        Remove-Item Env:AHAB_VISUAL_STATE -ErrorAction SilentlyContinue
                    } else {
                        $env:AHAB_VISUAL_STATE = $case.State
                    }
                    $process = Start-Process -FilePath $Executable -WorkingDirectory (Split-Path $Executable) -PassThru
                    try {
                        # GPUI loads fonts and assets during the first frame. Keep
                        # this delay explicit so captures are not startup frames.
                        Start-Sleep -Seconds 2
                        $output = Join-Path $OutputDirectory "gpui-$languageShort-$theme-$($case.Suffix)-$size.png"
                        & python $capture --pid $($process.Id) --output $output --logical-width $logicalWidth --logical-height $logicalHeight --settle-ms 800
                        if ($LASTEXITCODE -ne 0) {
                            throw "capture failed with exit code ${LASTEXITCODE}: $output"
                        }
                        Write-Host "captured $output"
                    } finally {
                        if (Get-Process -Id $process.Id -ErrorAction SilentlyContinue) {
                            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
                        }
                        Start-Sleep -Milliseconds 300
                    }
                }
            }
        }
    }
} finally {
    Stop-GpuiProcess
    Remove-Item Env:AHAB_VISUAL_THEME -ErrorAction SilentlyContinue
    Remove-Item Env:AHAB_VISUAL_LANGUAGE -ErrorAction SilentlyContinue
    Remove-Item Env:AHAB_VISUAL_ACCENT -ErrorAction SilentlyContinue
    Remove-Item Env:AHAB_VISUAL_PAGE -ErrorAction SilentlyContinue
    Remove-Item Env:AHAB_VISUAL_STATE -ErrorAction SilentlyContinue
}
