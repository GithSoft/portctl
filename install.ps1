# portctl Windows standalone installer
# https://github.com/tackelua/portctl
#
# Usage (PowerShell):
#   irm https://raw.githubusercontent.com/tackelua/portctl/main/install.ps1 | iex
#
$ErrorActionPreference = "Stop"

# Ensure TLS 1.2 is active for PowerShell 5.1
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$repo = "tackelua/portctl"
$tag = $env:PORTCTL_VERSION

if (-not $tag) {
    Write-Host "==> Querying latest release from GitHub..." -ForegroundColor Cyan
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -UseBasicParsing
        $tag = $release.tag_name
    } catch {
        $tag = "v2.1.0"
    }
}

if ($tag -and -not $tag.StartsWith("v")) {
    $tag = "v$tag"
}

$target = "x86_64-pc-windows-msvc"
$archiveName = "portctl-$tag-$target.zip"
$downloadUrl = "https://github.com/$repo/releases/download/$tag/$archiveName"
$checksumUrl = "$downloadUrl.sha256"

$installDir = if ($env:PORTCTL_INSTALL_DIR) {
    $env:PORTCTL_INSTALL_DIR
} elseif ($env:LOCALAPPDATA) {
    Join-Path $env:LOCALAPPDATA "Programs\portctl"
} else {
    Join-Path $env:USERPROFILE "bin"
}

Write-Host "==> Installing portctl $tag to $installDir..." -ForegroundColor Cyan

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
    $zipPath = Join-Path $tempDir $archiveName
    Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UseBasicParsing

    # Verify SHA256 Checksum
    $checksumPath = Join-Path $tempDir "$archiveName.sha256"
    try {
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing -ErrorAction Stop
        $expectedHash = ((Get-Content -Path $checksumPath -Raw) -split '\s+')[0].Trim().ToLower()
        $actualHash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
        if ($expectedHash -and $expectedHash -ne $actualHash) {
            throw "SHA256 checksum mismatch! Expected: $expectedHash, got: $actualHash"
        }
        Write-Host "==> Checksum verified." -ForegroundColor Cyan
    } catch {
        if ($_.Exception.Message -like "*SHA256 checksum mismatch*") {
            throw $_
        }
        Write-Warning "Could not verify checksum: $_"
    }

    Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item -Path (Join-Path $tempDir "portctl.exe") -Destination (Join-Path $installDir "portctl.exe") -Force

    # Add to persistent User PATH
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $paths = ($userPath -split ';') | Where-Object { $_ }
    if ($paths -notcontains $installDir) {
        $newUserPath = ($paths + $installDir) -join ';'
        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
        Write-Host "Notice: Added $installDir to persistent User PATH." -ForegroundColor Yellow
    }

    # Add to current PowerShell process session PATH
    if (($env:PATH -split ';') -notcontains $installDir) {
        $env:PATH = "$env:PATH;$installDir"
    }

    Write-Host "==> portctl $tag installed successfully!" -ForegroundColor Green
    & (Join-Path $installDir "portctl.exe") --version
} finally {
    Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
