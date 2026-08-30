param(
    [Parameter(Mandatory = $true)]
    [int]$GpuiPid,
    [int]$BackendPid,
    [string]$Scenario = "unspecified",
    [int]$DurationSeconds = 60,
    [int]$SampleIntervalMilliseconds = 1000,
    [string]$OutputPath = "artifacts/perf/gpui-resources.csv"
)

$ErrorActionPreference = "Stop"

function Get-GpuUsage {
    param([int]$Pid)

    $usage = @{ "3D" = 0.0; "Copy" = 0.0; "VideoDecode" = 0.0; "Other" = 0.0 }
    try {
        $samples = (Get-Counter '\GPU Engine(*)\Utilization Percentage').CounterSamples
        foreach ($sample in $samples) {
            if ($sample.InstanceName -notmatch "pid_$Pid(?:_|$)") {
                continue
            }
            $engine = if ($sample.InstanceName -match 'engtype_([^_]+)') {
                $Matches[1]
            } else {
                "Other"
            }
            if (-not $usage.ContainsKey($engine)) {
                $engine = "Other"
            }
            $usage[$engine] += [double]$sample.CookedValue
        }
    } catch {
        # GPU Engine counters are unavailable on some remote/session hosts.
        # Keep the row and mark GPU values as zero rather than losing CPU data.
    }
    return $usage
}

function Get-ProcessSnapshot {
    param([int]$Pid)

    try {
        $process = Get-Process -Id $Pid -ErrorAction Stop
        return @{
            ProcessCpuSeconds = [double]$process.TotalProcessorTime.TotalSeconds
            WorkingSetMb = [math]::Round($process.WorkingSet64 / 1MB, 2)
        }
    } catch {
        return $null
    }
}

$outputDirectory = Split-Path -Parent $OutputPath
if ($outputDirectory) {
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
}

$rows = [System.Collections.Generic.List[object]]::new()
$previous = @{}
$stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
$sampleCount = [math]::Max(1, [math]::Ceiling($DurationSeconds * 1000 / $SampleIntervalMilliseconds))

for ($index = 0; $index -lt $sampleCount; $index++) {
    $timestamp = [DateTime]::UtcNow
    foreach ($entry in @(@{ Name = "gpui"; Pid = $GpuiPid }, @{ Name = "backend"; Pid = $BackendPid })) {
        $snapshot = Get-ProcessSnapshot $entry.Pid
        $cpuPercent = $null
        $memoryMb = $null
        if ($null -ne $snapshot) {
            $memoryMb = $snapshot.WorkingSetMb
            if ($previous.ContainsKey($entry.Name)) {
                $old = $previous[$entry.Name]
                $wallSeconds = ($timestamp - $old.Timestamp).TotalSeconds
                if ($wallSeconds -gt 0) {
                    $cpuPercent = [math]::Round(
                        (($snapshot.ProcessCpuSeconds - $old.ProcessCpuSeconds) / $wallSeconds) *
                            100 / [Environment]::ProcessorCount,
                        3
                    )
                }
            }
            $previous[$entry.Name] = @{
                Timestamp = $timestamp
                ProcessCpuSeconds = $snapshot.ProcessCpuSeconds
            }
        }

        $gpu = Get-GpuUsage $entry.Pid
        $rows.Add([pscustomobject]@{
            TimestampUtc = $timestamp.ToString("o")
            Scenario = $Scenario
            Process = $entry.Name
            Pid = $entry.Pid
            CpuPercentOfMachine = $cpuPercent
            WorkingSetMb = $memoryMb
            Gpu3DPercent = [math]::Round($gpu["3D"], 3)
            GpuCopyPercent = [math]::Round($gpu["Copy"], 3)
            GpuVideoDecodePercent = [math]::Round($gpu["VideoDecode"], 3)
            GpuOtherPercent = [math]::Round($gpu["Other"], 3)
        })
    }

    $remaining = $SampleIntervalMilliseconds - [int]$stopwatch.ElapsedMilliseconds % $SampleIntervalMilliseconds
    if ($index -lt ($sampleCount - 1) -and $remaining -gt 0) {
        Start-Sleep -Milliseconds $remaining
    }
}

$rows | Export-Csv -NoTypeInformation -Encoding UTF8 -Path $OutputPath
Write-Host "Wrote $($rows.Count) samples to $OutputPath"
