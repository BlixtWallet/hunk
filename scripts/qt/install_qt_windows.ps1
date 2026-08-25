param(
    [Parameter(Mandatory = $true)]
    [string]$InstallRoot
)

$ErrorActionPreference = "Stop"

$qtVersion = "6.11.2"
$qtPackage = "qt.qt6.6112.win64_msvc2022_64"
$installerVersion = "4.11.0"
$installerSha256 = "ae919bc9b224b8ccdada69ec787a9f69330001f227f3fcbfb4a11a4adb3786f6"
$qtRoot = Join-Path $InstallRoot "$qtVersion\msvc2022_64"
$qmake = Join-Path $qtRoot "bin\qmake.exe"

function Export-QtEnvironment {
    param([string]$Root, [string]$QmakePath)

    if ($env:GITHUB_ENV) {
        "HUNK_QT_ROOT=$Root" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
        "QT_ROOT_DIR=$Root" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
        "QMAKE=$QmakePath" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
    }
    if ($env:GITHUB_PATH) {
        (Join-Path $Root "bin") | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
    }
}

if (Test-Path $qmake) {
    $installedVersion = (& $qmake -query QT_VERSION).Trim()
    if ($installedVersion -eq $qtVersion) {
        Export-QtEnvironment -Root $qtRoot -QmakePath $qmake
        Write-Host "Qt $installedVersion: $qtRoot"
        exit 0
    }
}

New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
$downloadRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { $env:TEMP }
$installer = Join-Path $downloadRoot "qt-online-installer-windows-x64-$installerVersion.exe"
$installerUrl = "https://download.qt.io/archive/online_installers/4.11/qt-online-installer-windows-x64-$installerVersion.exe"

Invoke-WebRequest -Uri $installerUrl -OutFile $installer
$actualInstallerSha256 = (Get-FileHash -Path $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualInstallerSha256 -ne $installerSha256) {
    throw "Qt installer checksum mismatch: expected $installerSha256, found $actualInstallerSha256"
}

& $installer `
    --root $InstallRoot `
    --accept-licenses `
    --default-answer `
    --confirm-command `
    install $qtPackage
if ($LASTEXITCODE -ne 0) {
    throw "Qt Online Installer exited with status $LASTEXITCODE"
}

if (-not (Test-Path $qmake)) {
    throw "Qt installation did not create $qmake"
}
$installedVersion = (& $qmake -query QT_VERSION).Trim()
if ($installedVersion -ne $qtVersion) {
    throw "Hunk requires Qt $qtVersion, found $installedVersion"
}

Export-QtEnvironment -Root $qtRoot -QmakePath $qmake
Write-Host "Qt $installedVersion: $qtRoot"
