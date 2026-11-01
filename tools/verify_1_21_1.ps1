$ErrorActionPreference = "Stop"

$repoRoot = (git rev-parse --show-toplevel 2>$null)
if (-not $repoRoot) {
    throw "Run this script from inside the Nexora repository."
}
Set-Location $repoRoot

$activeBranch = git branch --show-current
if (-not $activeBranch) {
    throw "Run this verifier from a named local branch, not a detached HEAD."
}

$logDir = Join-Path $repoRoot "target"
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$logPath = Join-Path $logDir "1.21.1-verification.log"
Start-Transcript -Path $logPath -Force | Out-Null

function Invoke-Checked {
    param([string]$Label, [scriptblock]$Command)
    Write-Host "`n=== $Label ===" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE. See $logPath"
    }
}

try {
    Write-Host "Using the current local checkout. This verifier does not pull, commit, or push."

    $jdk21 = Join-Path $env:LOCALAPPDATA "Programs\Eclipse Adoptium\jdk-21.0.13.9-hotspot"
    if (Test-Path (Join-Path $jdk21 "bin\java.exe")) {
        $env:JAVA_HOME = $jdk21
        $env:Path = (Join-Path $jdk21 "bin") + ";" + $env:Path
    }

    Invoke-Checked "Python mapping, packet-audit, and version-port tests" { python -m unittest tools.test_proguard tools.test_packet_codec_coverage tools.test_version_port tools.test_builtin_registry_inventory tools.test_1_21_1_block_facts }
    Invoke-Checked "Rust formatting" { cargo fmt --all -- --check }
    Invoke-Checked "Core version tests" { cargo test -p horizon-core }
    Invoke-Checked "Rust protocol tests" { cargo test -p horizon-protocol }
    Invoke-Checked "World registry tests" { cargo test -p horizon-world }
    Invoke-Checked "Workspace compile check" { cargo check --workspace }

    $reportCandidates = @()
    if ($env:NEXORA_1_21_1_REPORT) {
        $reportCandidates += $env:NEXORA_1_21_1_REPORT
    }
    $reportCandidates += (Join-Path $env:USERPROFILE "code\horizon-parity\versions\1.21.1\datagen\reports\packets.json")
    $report = $reportCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

    if ($report) {
        Invoke-Checked "Generate Minecraft 1.21.1 packet IDs" {
            python tools/generate_packet_ids.py --report $report --out "ports/1.21.1/packet_ids.rs" --protocol-version 767 --version-name 1.21.1
        }
        Invoke-Checked "Audit target packet codec coverage" {
            python tools/audit_packet_codec_coverage.py
        }
        Write-Host "Generated packet inventory and audit locally; no commit or push was performed."
    } elseif (Test-Path "ports/1.21.1/packet_ids.rs") {
        Invoke-Checked "Audit target packet codec coverage" {
            python tools/audit_packet_codec_coverage.py
        }
        Write-Host "Updated packet codec coverage audit locally; no commit or push was performed."
    } else {
        Write-Warning "Skipped packet-ID generation/audit: no local 1.21.1 packets.json found. Set NEXORA_1_21_1_REPORT to its full path to enable it."
    }

    Write-Host "`nVerification completed. Full output: $logPath" -ForegroundColor Green
} catch {
    Write-Error $_
    Write-Host "Verification stopped. Full output: $logPath" -ForegroundColor Yellow
    exit 1
} finally {
    Stop-Transcript | Out-Null
}
