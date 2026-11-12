# Compare the packaged NSH app with the GPUI client: time to the first window,
# then idle memory and CPU.
#
# Both run on the same machine, at 820x680, with the mock backend (the GPUI
# client is told AHAB_BACKEND=mock so no Python sidecar starts), light theme.
# A run is discarded if a window does not appear within the timeout.
#
#   pwsh scripts/benchmark_nsh_vs_gpui.ps1
#   pwsh scripts/benchmark_nsh_vs_gpui.ps1 -Runs 10 -Out artifacts/nsh/perf.json
[CmdletBinding()]
param(
    [string]$NshExe = "artifacts/nsh/package/AALC/AALC.exe",
    [string]$GpuiExe = "gpui-app/target/release/ahab-gpui-app.exe",
    [int]$Runs = 5,
    [int]$SettleMs = 4000,
    [int]$TimeoutSeconds = 30,
    [string]$Out = "artifacts/nsh/perf.json"
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Resolve-App([string]$path) {
    $full = if ([System.IO.Path]::IsPathRooted($path)) { $path } else { Join-Path $repo $path }
    if (-not (Test-Path $full)) { throw "not found: $full" }
    return (Resolve-Path $full).Path
}

$NshExe = Resolve-App $NshExe
$GpuiExe = Resolve-App $GpuiExe
$Out = if ([System.IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $repo $Out }
New-Item -ItemType Directory -Force -Path (Split-Path $Out) | Out-Null

function Stop-AppProcesses {
    Get-Process -Name "AALC", "ahab-gpui-app", "nsh-windows-host", "nsh-windows-app" -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
}

function Wait-MainWindow($process, [int]$timeoutSeconds) {
    $deadline = [DateTime]::UtcNow.AddSeconds($timeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($process.HasExited) { throw "process exited before opening a window" }
        $process.Refresh()
        if ($process.MainWindowHandle -ne [IntPtr]::Zero) { return $process.MainWindowHandle }
        Start-Sleep -Milliseconds 10
    }
    throw "no main window within ${timeoutSeconds}s"
}

function Measure-App([string]$name, [string]$exe, [hashtable]$environment) {
    $samples = @()
    for ($run = 1; $run -le $Runs; $run++) {
        Stop-AppProcesses
        foreach ($key in $environment.Keys) {
            [Environment]::SetEnvironmentVariable($key, $environment[$key], "Process")
        }
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        $process = Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe) -PassThru
        try {
            [void](Wait-MainWindow $process $TimeoutSeconds)
            $startupMs = $stopwatch.Elapsed.TotalMilliseconds
            Start-Sleep -Milliseconds $SettleMs
            $process.Refresh()
            $cpu = $process.TotalProcessorTime.TotalMilliseconds
            $samples += [pscustomobject]@{
                app          = $name
                run          = $run
                startup_ms   = [math]::Round($startupMs, 1)
                working_set  = [math]::Round($process.WorkingSet64 / 1MB, 1)
                private      = [math]::Round($process.PrivateMemorySize64 / 1MB, 1)
                paged        = [math]::Round($process.PagedMemorySize64 / 1MB, 1)
                cpu_ms       = [math]::Round($cpu, 1)
                threads      = $process.Threads.Count
                handles      = $process.HandleCount
            }
        } finally {
            if (-not $process.HasExited) {
                [void]$process.CloseMainWindow()
                if (-not $process.WaitForExit(3000)) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue }
            }
            Stop-AppProcesses
        }
    }
    return $samples
}

$nshEnv = @{ AALC_VISUAL_THEME = "light" }
$gpuiEnv = @{
    AHAB_BACKEND       = "mock"
    AHAB_VISUAL_THEME  = "light"
    AHAB_VISUAL_PAGE   = "home"
    AHAB_VISUAL_LANGUAGE = "zh-CN"
    AHAB_VISUAL_ACCENT = "crimson"
    AHAB_VISUAL_SKIN   = "default"
}

$rows = @()
$rows += Measure-App "nsh" $NshExe $nshEnv
$rows += Measure-App "gpui" $GpuiExe $gpuiEnv

$summary = $rows | Group-Object app | ForEach-Object {
    $items = $_.Group
    [pscustomobject]@{
        app              = $_.Name
        runs             = $items.Count
        startup_ms_avg   = [math]::Round(($items | Measure-Object startup_ms -Average).Average, 1)
        startup_ms_min   = ($items | Measure-Object startup_ms -Minimum).Minimum
        startup_ms_max   = ($items | Measure-Object startup_ms -Maximum).Maximum
        working_set_mb   = [math]::Round(($items | Measure-Object working_set -Average).Average, 1)
        private_mb       = [math]::Round(($items | Measure-Object private -Average).Average, 1)
        cpu_ms           = [math]::Round(($items | Measure-Object cpu_ms -Average).Average, 1)
        threads          = [math]::Round(($items | Measure-Object threads -Average).Average, 1)
        handles          = [math]::Round(($items | Measure-Object handles -Average).Average, 1)
    }
}

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToString("o")
    runs_per_app = $Runs
    settle_ms    = $SettleMs
    exe = @{ nsh = $NshExe; gpui = $GpuiExe }
    summary = $summary
    samples = $rows
}
$report | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 $Out

$summary | Format-Table -AutoSize
Write-Host "wrote $Out"