$ErrorActionPreference = "Stop"

$repoRoot = (git rev-parse --show-toplevel 2>$null)
if (-not $repoRoot) {
    throw "Run this script from inside the Nexora repository."
}
Set-Location $repoRoot

$expectedBranch = "nexora/master"
$activeBranch = git branch --show-current
if ($activeBranch -ne $expectedBranch) {
    throw "This verifier must run on '$expectedBranch'. Current branch: '$activeBranch'. Switch branches first; no files were changed."
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
    Invoke-Checked "Update branch" { git pull --ff-only }

    $jdk21 = Join-Path $env:LOCALAPPDATA "Programs\Eclipse Adoptium\jdk-21.0.13.9-hotspot"
    if (Test-Path (Join-Path $jdk21 "bin\java.exe")) {
        $env:JAVA_HOME = $jdk21
        $env:Path = (Join-Path $jdk21 "bin") + ";" + $env:Path
    }

    Invoke-Checked "Python mapping tests" { python -m unittest tools.test_proguard }
    Invoke-Checked "Rust protocol tests" { cargo test -p horizon-protocol }
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
        Invoke-Checked "Publish generated packet inventory and audit" {
            git add "ports/1.21.1/packet_ids.rs" "ports/1.21.1/packet-codec-coverage.json"
            git diff --cached --quiet
            if ($LASTEXITCODE -ne 0) {
                git commit -m "Add generated Minecraft 1.21.1 packet inventory and codec audit"
                if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
                git push
            }
        }
    } elseif (Test-Path "ports/1.21.1/packet_ids.rs") {
        Invoke-Checked "Audit target packet codec coverage" {
            python tools/audit_packet_codec_coverage.py
        }
        Invoke-Checked "Publish packet codec coverage audit" {
            git add "ports/1.21.1/packet-codec-coverage.json"
            git diff --cached --quiet
            if ($LASTEXITCODE -ne 0) {
                git commit -m "Update Minecraft 1.21.1 packet codec coverage audit"
                if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
                git push
            }
        }
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
