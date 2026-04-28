$ErrorActionPreference = "Stop"

$Repo = if ($env:DSW_REPO) { $env:DSW_REPO } else { "hqman/droid-switch" }
$Version = if ($env:DSW_VERSION) { $env:DSW_VERSION } else { "latest" }
$InstallDir = if ($env:DSW_INSTALL_DIR) {
    $env:DSW_INSTALL_DIR
} else {
    Join-Path $env:LOCALAPPDATA "dsw\bin"
}

$Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($Arch) {
    "X64" { $Target = "x86_64-pc-windows-msvc" }
    default { throw "unsupported Windows arch: $Arch" }
}

$Asset = "dsw-$Target.zip"
if ($Version -eq "latest") {
    $Url = "https://github.com/$Repo/releases/latest/download/$Asset"
} else {
    $Url = "https://github.com/$Repo/releases/download/$Version/$Asset"
}

$Temp = Join-Path ([System.IO.Path]::GetTempPath()) ("dsw-install-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $Temp | Out-Null

try {
    $Zip = Join-Path $Temp $Asset
    Write-Host "downloading $Url"
    Invoke-WebRequest -Uri $Url -OutFile $Zip
    Expand-Archive -Path $Zip -DestinationPath $Temp -Force

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $Exe = Get-ChildItem -Path $Temp -Recurse -Filter "dsw.exe" | Select-Object -First 1
    if (-not $Exe) {
        throw "dsw.exe not found in archive"
    }
    Copy-Item $Exe.FullName (Join-Path $InstallDir "dsw.exe") -Force

    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $PathParts = @()
    if ($UserPath) {
        $PathParts = $UserPath -split ";"
    }
    if ($PathParts -notcontains $InstallDir) {
        $NewPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "added to user PATH: $InstallDir"
    }

    Write-Host "installed: $(Join-Path $InstallDir 'dsw.exe')"
} finally {
    Remove-Item -Recurse -Force $Temp -ErrorAction SilentlyContinue
}
