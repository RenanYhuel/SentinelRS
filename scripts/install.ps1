$ErrorActionPreference = "Stop"

$Repo = "sentinelrs/sentinelrs"
$Binary = "sentinel_cli.exe"
$InstallName = "sentinel.exe"

function Get-LatestVersion {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    return $release.tag_name
}

function Install-Sentinel {
    param([string]$Version)

    if (-not $Version) {
        Write-Host "[info]  Fetching latest version..." -ForegroundColor Cyan
        $Version = Get-LatestVersion
    }

    Write-Host "[info]  Installing SentinelRS CLI $Version for windows/amd64" -ForegroundColor Cyan

    $AssetName = "sentinel-windows-amd64.zip"
    $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$AssetName"
    $ChecksumUrl = "https://github.com/$Repo/releases/download/$Version/SHA256SUMS.txt"

    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) "sentinel-install-$(Get-Random)"
    New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

    try {
        Write-Host "[info]  Downloading $DownloadUrl..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $DownloadUrl -OutFile "$TempDir\$AssetName" -UseBasicParsing

        Write-Host "[info]  Downloading checksums..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile "$TempDir\SHA256SUMS.txt" -UseBasicParsing

        Write-Host "[info]  Verifying checksum..." -ForegroundColor Cyan
        $ExpectedHash = (Get-Content "$TempDir\SHA256SUMS.txt" | Select-String $AssetName) -replace "\s+.*$", ""
        $ActualHash = (Get-FileHash "$TempDir\$AssetName" -Algorithm SHA256).Hash.ToLower()
        if ($ExpectedHash -and ($ActualHash -ne $ExpectedHash)) {
            Write-Host "[warn]  Checksum mismatch" -ForegroundColor Yellow
        }

        Write-Host "[info]  Extracting..." -ForegroundColor Cyan
        Expand-Archive -Path "$TempDir\$AssetName" -DestinationPath "$TempDir\extract" -Force

        $InstallDir = Join-Path $env:LOCALAPPDATA "SentinelRS\bin"
        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

        Copy-Item "$TempDir\extract\$Binary" "$InstallDir\$InstallName" -Force

        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($UserPath -notlike "*$InstallDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
            Write-Host "[ok]    Added $InstallDir to PATH" -ForegroundColor Green
        }

        Write-Host "[ok]    SentinelRS CLI installed to $InstallDir\$InstallName" -ForegroundColor Green
        Write-Host ""
        Write-Host "[info]  Quick start:" -ForegroundColor Cyan
        Write-Host "  sentinel init     - Initialize a new project"
        Write-Host "  sentinel up       - Start the stack"
        Write-Host "  sentinel status   - Check service status"
    }
    finally {
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    }
}

Install-Sentinel -Version $env:VERSION
