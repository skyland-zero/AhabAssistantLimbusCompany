[CmdletBinding()]
param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot "..\artifacts\avalonia\win-x64"),
    [switch]$KeepSymbols
)

$ErrorActionPreference = "Stop"

$projectPath = [IO.Path]::GetFullPath(
    (Join-Path $PSScriptRoot "..\avalonia-ui\AhabAssistant.Avalonia.csproj")
)
$publishDirectory = [IO.Path]::GetFullPath($OutputDirectory)
$symbolsDirectory = Join-Path (Split-Path $publishDirectory -Parent) "symbols\win-x64"

if (-not (Test-Path $projectPath -PathType Leaf)) {
    throw "Avalonia project not found: $projectPath"
}

# 清理发布目录，避免上一次发布残留 PDB 或其他文件混入本次产物。
if (Test-Path $publishDirectory) {
    Remove-Item $publishDirectory -Recurse -Force
}
New-Item $publishDirectory -ItemType Directory -Force | Out-Null

if ($KeepSymbols -and (Test-Path $symbolsDirectory)) {
    Remove-Item $symbolsDirectory -Recurse -Force
}
if ($KeepSymbols) {
    New-Item $symbolsDirectory -ItemType Directory -Force | Out-Null
}

$publishArguments = @(
    "publish",
    $projectPath,
    "--configuration", "Release",
    "-p:PublishProfile=win-x64-aot",
    "-p:PublishDir=$publishDirectory\",
    "--nologo",
    "--verbosity", "minimal"
)

# 让脚本在清理前拿到 PDB；它们不会进入最终用户目录。
if ($KeepSymbols) {
    $publishArguments += "-p:KeepPublishSymbols=true"
}

Write-Host "Publishing Avalonia UI (win-x64, Native AOT)..."
& dotnet @publishArguments
if ($LASTEXITCODE -ne 0) {
    throw "dotnet publish failed with exit code $LASTEXITCODE"
}

$pdbFiles = @(Get-ChildItem $publishDirectory -Recurse -File -Filter "*.pdb")
if ($KeepSymbols) {
    foreach ($pdb in $pdbFiles) {
        $relativePath = $pdb.FullName.Substring($publishDirectory.Length).TrimStart("\", "/")
        $symbolPath = Join-Path $symbolsDirectory $relativePath
        $symbolParent = Split-Path $symbolPath -Parent
        New-Item $symbolParent -ItemType Directory -Force | Out-Null
        Copy-Item $pdb.FullName $symbolPath -Force
    }
}

# PDB 只用于调试，不属于用户运行时文件；无论是否保留符号副本，都从用户目录删除。
$pdbFiles | Remove-Item -Force

$remainingPdbFiles = @(Get-ChildItem $publishDirectory -Recurse -File -Filter "*.pdb")
if ($remainingPdbFiles.Count -ne 0) {
    throw "Publish directory still contains PDB files."
}

$executables = @(Get-ChildItem $publishDirectory -Recurse -File -Filter "*.exe")
if ($executables.Count -eq 0) {
    throw "Publish directory does not contain an executable."
}

$size = (Get-ChildItem $publishDirectory -Recurse -File |
    Measure-Object -Property Length -Sum).Sum
$sizeMiB = [Math]::Round($size / 1MB, 2)

Write-Host "Publish directory: $publishDirectory"
Write-Host "Executable: $($executables[0].FullName)"
Write-Host "Payload size: $sizeMiB MiB"
if ($KeepSymbols) {
    Write-Host "Symbols directory: $symbolsDirectory"
}
