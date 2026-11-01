$ErrorActionPreference = "Stop"

$Repository = "RNGaming11/Nexora"
$Branch = "nexora/master"
$KeyPath = "crates/horizon-release-sign/release-keys.txt"
$Endpoint = "repos/$Repository/contents/$KeyPath"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "GitHub CLI is required. Install it, then run: gh auth login"
}
& gh auth status
if ($LASTEXITCODE -ne 0) {
    throw "Sign in first with: gh auth login"
}

$OpenSsl = Get-Command openssl -ErrorAction SilentlyContinue
if ($OpenSsl) {
    $OpenSslPath = $OpenSsl.Source
} else {
    $Candidates = @(
        (Join-Path $env:ProgramFiles "Git\usr\bin\openssl.exe"),
        (Join-Path $env:LOCALAPPDATA "Programs\Git\usr\bin\openssl.exe")
    )
    $OpenSslPath = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $OpenSslPath) {
    throw "OpenSSL was not found. Install Git for Windows or OpenSSL, then rerun this script."
}

$Temp = Join-Path $env:TEMP ("nexora-signing-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Temp | Out-Null
$PrivateFile = Join-Path $Temp "private.der"
$PublicFile = Join-Path $Temp "public.der"
$SecretFile = Join-Path $Temp "secret.txt"
$RequestFile = Join-Path $Temp "request.json"
$RollbackFile = Join-Path $Temp "rollback.json"

function Invoke-Gh {
    param([string[]]$Arguments)
    $Output = & gh @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "GitHub CLI command failed: gh $($Arguments -join ' ')"
    }
    return $Output
}

try {
    & $OpenSslPath genpkey -algorithm ED25519 -outform DER -out $PrivateFile
    if ($LASTEXITCODE -ne 0) { throw "Could not generate the Ed25519 signing key." }

    & $OpenSslPath pkey -inform DER -in $PrivateFile -pubout -outform DER -out $PublicFile
    if ($LASTEXITCODE -ne 0) { throw "Could not derive the Ed25519 public key." }

    $PublicBytes = [System.IO.File]::ReadAllBytes($PublicFile)
    if ($PublicBytes.Length -lt 32) { throw "OpenSSL returned an invalid public key." }
    $PublicKey = [Convert]::ToBase64String($PublicBytes[($PublicBytes.Length - 32)..($PublicBytes.Length - 1)])
    $PrivateKey = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($PrivateFile))

    $Info = (Invoke-Gh @("api", ($Endpoint + "?ref=" + $Branch))) | ConvertFrom-Json
    $OriginalText = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String(($Info.content -replace '\s', '')))
    $Comments = @($OriginalText -split "\r?\n" | Where-Object { $_.Trim() -eq "" -or $_.Trim().StartsWith("#") })
    $UpdatedText = (($Comments + $PublicKey) -join [char]10) + [char]10

    $Body = @{
        message = "Set Nexora release verification key"
        content = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($UpdatedText))
        sha = $Info.sha
        branch = $Branch
    } | ConvertTo-Json -Compress
    [IO.File]::WriteAllText($RequestFile, $Body, [Text.UTF8Encoding]::new($false))

    $NewSha = (Invoke-Gh @("api", "--method", "PUT", $Endpoint, "--input", $RequestFile, "--jq", ".content.sha")).Trim()
    if (-not $NewSha) { throw "GitHub did not confirm the public-key update." }

    [IO.File]::WriteAllText($SecretFile, $PrivateKey, [Text.UTF8Encoding]::new($false))
    $Process = Start-Process -FilePath (Get-Command gh).Source -ArgumentList @("secret", "set", "RELEASE_SIGNING_KEY", "--env", "release") -RedirectStandardInput $SecretFile -NoNewWindow -Wait -PassThru
    if ($Process.ExitCode -ne 0) {
        $RollbackBody = @{
            message = "Restore Nexora release verification key after secret setup failure"
            content = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($OriginalText))
            sha = $NewSha
            branch = $Branch
        } | ConvertTo-Json -Compress
        [IO.File]::WriteAllText($RollbackFile, $RollbackBody, [Text.UTF8Encoding]::new($false))
        try {
            Invoke-Gh @("api", "--method", "PUT", $Endpoint, "--input", $RollbackFile) | Out-Null
        } catch {
            Write-Warning "Automatic rollback failed. The release key file was updated, but the GitHub secret was not set."
        }
        throw "Could not set the release environment secret. Ensure the GitHub environment named 'release' exists and your account can manage its secrets."
    }

    Write-Host "Nexora release signing is configured."
    Write-Host "The private key was stored only as the GitHub 'release' environment secret; the local temporary files will be deleted."
} finally {
    Remove-Item -LiteralPath $Temp -Recurse -Force -ErrorAction SilentlyContinue
}
