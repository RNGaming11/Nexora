param(
    [string]$Output = ".\dist\nexora-1.21.1-local"
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Release = Join-Path $Root "target\release"
$Destination = Join-Path $Root $Output
$Required = @(
    "horizon-client-1-21-1.exe",
    "horizon-ctl.exe",
    "horizon-prepare.exe"
)

foreach ($Name in $Required) {
    $Path = Join-Path $Release $Name
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing $Path. Build the 1.21.1 client and workspace release binaries first."
    }
}

if (Test-Path -LiteralPath $Destination) {
    Remove-Item -LiteralPath $Destination -Recurse -Force
}
New-Item -ItemType Directory -Path $Destination -Force | Out-Null

foreach ($Name in $Required) {
    Copy-Item -LiteralPath (Join-Path $Release $Name) -Destination $Destination
}

$PrepareSources = Join-Path $Root "crates\horizon-prepare"
$ExporterSources = Join-Path $Root "mods\horizon\src\test\java\dev\horizonlab\parity"
if (-not (Test-Path -LiteralPath $PrepareSources -PathType Container)) {
    throw "Missing horizon-prepare sources: $PrepareSources"
}
if (-not (Test-Path -LiteralPath $ExporterSources -PathType Container)) {
    throw "Missing vanilla exporter sources: $ExporterSources"
}

New-Item -ItemType Directory -Path (Join-Path $Destination "crates") -Force | Out-Null
Copy-Item -LiteralPath $PrepareSources -Destination (Join-Path $Destination "crates") -Recurse
$ExporterRoot = Join-Path $Destination "mods\horizon\src\test\java\dev\horizonlab"
New-Item -ItemType Directory -Path $ExporterRoot -Force | Out-Null
Copy-Item -LiteralPath $ExporterSources -Destination $ExporterRoot -Recurse

Copy-Item -LiteralPath (Join-Path $Root "ports\1.21.1\research-lock.json") -Destination (Join-Path $Destination "research-lock.json")
Copy-Item -LiteralPath (Join-Path $Root "ports\1.21.1\target.json") -Destination (Join-Path $Destination "target.json")
Copy-Item -LiteralPath (Join-Path $Root "ports\1.21.1") -Destination (Join-Path $Destination "ports") -Recurse

$Lock = Get-Content -LiteralPath (Join-Path $Destination "research-lock.json") -Raw | ConvertFrom-Json
if ($Lock.minecraft.version -ne "1.21.1" -or $Lock.minecraft.java_major -ne 21) {
    throw "The packaged research lock does not target Minecraft 1.21.1 with Java 21."
}

Write-Host "Created local launcher package: $Destination"
Write-Host "Minecraft target: $($Lock.minecraft.version)"
Write-Host "Client: $((Get-Item (Join-Path $Destination "horizon-client-1-21-1.exe")).Length) bytes"
Write-Host "This package is not a claim that the 1.21.1 gameplay port is complete."
