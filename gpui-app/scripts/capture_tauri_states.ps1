[CmdletBinding()]
param(
    [string]$OutputDirectory = "",
    [ValidateSet("900x680", "800x560")]
    [string[]]$Sizes = @("900x680", "800x560"),
    [ValidateSet("zh-CN", "en-US")]
    [string[]]$Languages = @("zh-CN", "en-US"),
    [ValidateSet("light", "dark")]
    [string[]]$Themes = @("light", "dark"),
    [string[]]$States = @("home-expanded", "home-select", "home-running", "home-paused", "home-after-completion", "teams-editor", "teams-delete", "teams-select", "settings-hotkey", "settings-select", "settings-latest", "toolbox-running", "resources-syncing", "help-scrolled")
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repo "artifacts/visual/reference-ui-states"
} else {
    $OutputDirectory = (Resolve-Path $OutputDirectory -ErrorAction SilentlyContinue)?.Path ?? (Join-Path (Get-Location) $OutputDirectory)
}
New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$capture = (Resolve-Path (Join-Path $PSScriptRoot "../tools/capture_window.py")).Path
$log = Join-Path $OutputDirectory "tauri-dev.log"
$errLog = Join-Path $OutputDirectory "tauri-dev.log.err"
Remove-Item $log, $errLog -Force -ErrorAction SilentlyContinue
$validStates = @("home-expanded", "home-select", "home-running", "home-paused", "home-after-completion", "teams-editor", "teams-delete", "teams-select", "settings-hotkey", "settings-select", "settings-latest", "toolbox-running", "resources-syncing", "help-scrolled")
$States = @($States | ForEach-Object { $_ -split "," } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$unknownStates = @($States | Where-Object { $_ -notin $validStates })
if ($unknownStates.Count -gt 0) {
    throw "unknown visual state(s): $($unknownStates -join ', ')"
}

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

function Invoke-JavaScript {
    param([Parameter(Mandatory = $true)][string]$Source)
    $bytes = [Text.Encoding]::UTF8.GetBytes($Source)
    $encoded = [Convert]::ToBase64String($bytes)
    Invoke-AgentBrowser eval -b $encoded | Out-Null
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
            # WebView2 exposes CDP only after Tauri has created the window.
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

function Set-TauriAppearance {
    param(
        [string]$Page,
        [string]$Language,
        [string]$Theme
    )
    $state = @{
        currentPage = $Page
        sidebarCollapsed = $false
        rightPanelWidth = 280
        rightPanelCollapsed = $false
        themeMode = $Theme
        accentId = "crimson"
        language = $Language
    } | ConvertTo-Json -Compress
    $stateEnvelope = @{ state = (ConvertFrom-Json $state); version = 0 } | ConvertTo-Json -Compress
    $bytes = [Text.Encoding]::UTF8.GetBytes($stateEnvelope)
    $encoded = [Convert]::ToBase64String($bytes)
    Invoke-JavaScript "localStorage.setItem('ahab-ui-settings', atob('$encoded')); location.reload(); 'reload'"
    # Wait for the SPA and its mock IPC requests, not merely the WebView load.
    Invoke-AgentBrowser wait 1200 | Out-Null
    Start-Sleep -Milliseconds 500
}

function Get-StateAction {
    param(
        [string]$State,
        [string]$Language
    )
    $isZh = $Language -eq "zh-CN"
    $dailyTitle = if ($isZh) { "日常任务" } else { "Daily Tasks" }
    $rewardTitle = if ($isZh) { "领取奖励" } else { "Claim Rewards" }
    $afterHint = if ($isZh) { "点击配置所有任务执行完成后的收尾与电源动作" } else { "Configure actions and power state after all tasks finish" }
    $editLabel = if ($isZh) { "编辑队伍" } else { "Edit Team" }
    $deleteLabel = if ($isZh) { "删除" } else { "Delete" }
    $pauseLabel = if ($isZh) { "暂停" } else { "Pause" }
    $checkLabel = if ($isZh) { "检查更新" } else { "Check for Updates" }
    $runLabel = if ($isZh) { "运行" } else { "Run" }
    $syncLabel = if ($isZh) { "立即同步" } else { "Sync Now" }

    $daily = $dailyTitle | ConvertTo-Json -Compress
    $reward = $rewardTitle | ConvertTo-Json -Compress
    $hint = $afterHint | ConvertTo-Json -Compress
    $edit = $editLabel | ConvertTo-Json -Compress
    $delete = $deleteLabel | ConvertTo-Json -Compress
    $pause = $pauseLabel | ConvertTo-Json -Compress
    $check = $checkLabel | ConvertTo-Json -Compress
    $run = $runLabel | ConvertTo-Json -Compress
    $sync = $syncLabel | ConvertTo-Json -Compress

    return @"
(() => {
  const text = (node) => (node?.textContent || '').replace(/\\s+/g, ' ').trim();
  const exact = (value) => Array.from(document.querySelectorAll('span,p,div')).find((node) => node.children.length === 0 && text(node) === value);
  const buttonContaining = (value) => Array.from(document.querySelectorAll('button')).find((node) => text(node).includes(value));
  const attrContaining = (name, value) => Array.from(document.querySelectorAll('button')).find((node) => (node.getAttribute(name) || '').includes(value));
  const role = (name, index = 0) => Array.from(document.querySelectorAll('[role="' + name + '"]'))[index];
  const click = (node, description) => { if (!node) throw new Error('visual state target not found: ' + description); node.click(); };
  const clickExact = (value) => click(exact(value), value);
  const clickButton = (value) => click(buttonContaining(value), 'button ' + value);
  switch ('$State') {
    case 'home-expanded':
      clickExact($daily);
      break;
    case 'home-select':
      clickExact($reward);
      setTimeout(() => click(role('combobox'), 'reward select'), 250);
      break;
    case 'home-running':
      clickButton('Link Start!');
      break;
    case 'home-paused':
      clickButton('Link Start!');
      setTimeout(() => clickButton($pause), 450);
      break;
    case 'home-after-completion':
      click(attrContaining('title', $hint), 'after completion button');
      break;
    case 'teams-editor':
      click(document.querySelector('button[aria-label="' + $edit + '"]'), 'edit team');
      break;
    case 'teams-delete':
      click(document.querySelector('button[aria-label="' + $delete + '"]'), 'delete team');
      break;
    case 'teams-select':
      click(document.querySelector('button[aria-label="' + $edit + '"]'), 'edit team');
      setTimeout(() => click(role('combobox'), 'team select'), 300);
      break;
    case 'settings-hotkey':
      clickButton('F10');
      break;
    case 'settings-select':
      click(role('combobox'), 'settings select');
      break;
    case 'settings-latest':
      clickButton($check);
      break;
    case 'toolbox-running':
      click(Array.from(document.querySelectorAll('button')).find((node) => text(node) === $run), 'tool run');
      break;
    case 'resources-syncing':
      clickButton($sync);
      break;
    case 'help-scrolled':
      {
        const documentScroll = document.querySelector('[data-slot="scroll-area-viewport"]');
        if (!documentScroll) throw new Error('help document scroll target not found');
        documentScroll.scrollTop = Math.max(260, documentScroll.scrollHeight / 3);
        documentScroll.dispatchEvent(new Event('scroll', { bubbles: true }));
      }
      break;
    default:
      throw new Error('unknown visual state: $State');
  }
  'state applied'
})()
"@
}

$tauri = $null
try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9222"
    $tauri = Start-Process -FilePath "pnpm.cmd" -ArgumentList "--dir", "ui", "tauri", "dev" -WorkingDirectory $repo -RedirectStandardOutput $log -RedirectStandardError $errLog -PassThru
    Wait-TauriTarget
    # The target is now available, but the first React/IPC frame may still be
    # loading. Never capture that startup frame.
    Start-Sleep -Seconds 2

    foreach ($size in $Sizes) {
        $parts = $size -split "x"
        $logicalWidth = [int]$parts[0]
        $logicalHeight = [int]$parts[1]
        Resize-TauriClient $logicalWidth $logicalHeight
        foreach ($language in $Languages) {
            $languageShort = if ($language -eq "zh-CN") { "zh" } else { "en" }
            foreach ($theme in $Themes) {
                foreach ($visualState in $States) {
                    $page = switch -Regex ($visualState) {
                        '^home-' { 'home'; break }
                        '^teams-' { 'teams'; break }
                        '^settings-' { 'settings'; break }
                        '^toolbox-' { 'toolbox'; break }
                        '^resources-' { 'resources'; break }
                        '^help-' { 'help'; break }
                        default { throw "no page mapping for state $visualState" }
                    }
                    Set-TauriAppearance $page $language $theme
                    $action = Get-StateAction $visualState $language
                    Invoke-JavaScript $action
                    # Allow state updates, delayed modal rendering, and resource
                    # progress events to settle before the native-pixel capture.
                    Invoke-AgentBrowser wait 900 | Out-Null
                    Start-Sleep -Milliseconds 300
                    $output = Join-Path $OutputDirectory "tauri-$languageShort-$theme-$visualState-$size.png"
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
