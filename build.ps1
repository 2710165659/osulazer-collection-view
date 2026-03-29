Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $projectRoot

$appName = "osulazer-collection-view"
$buildRoot = Join-Path $projectRoot "build"
$pyinstallerWork = Join-Path $buildRoot "pyinstaller"
$distRoot = Join-Path $projectRoot "dist"
$distApp = Join-Path $distRoot $appName
$legacyDistApp = Join-Path $distRoot "collection-view"

Write-Host "Installing Python dependencies..."
python -m pip install -r requirements.txt pyinstaller

Write-Host "Checking bundled extractor runtime..."
$extractorExe = Join-Path $projectRoot "extractor\\extractor.exe"
$extractorDll = Join-Path $projectRoot "extractor\\realm-wrappers.dll"
if (-not (Test-Path $extractorExe)) {
    throw "Missing extractor\\extractor.exe"
}
if (-not (Test-Path $extractorDll)) {
    throw "Missing extractor\\realm-wrappers.dll"
}

Write-Host "Cleaning old packaged app..."
if (Test-Path $distApp) {
    Remove-Item $distApp -Recurse -Force
}
if ((Test-Path $legacyDistApp) -and ($legacyDistApp -ne $distApp)) {
    Remove-Item $legacyDistApp -Recurse -Force
}
if (Test-Path $pyinstallerWork) {
    Remove-Item $pyinstallerWork -Recurse -Force
}

Write-Host "Building Python desktop app..."
python -m PyInstaller osulazer-collection-view.spec `
    --noconfirm `
    --clean `
    --distpath $distRoot `
    --workpath $pyinstallerWork

$runtimeDir = Join-Path $distApp "runtime"
$coversDir = Join-Path $runtimeDir "covers"
New-Item -ItemType Directory -Force -Path $coversDir | Out-Null
Copy-Item README.md (Join-Path $distApp "README.md") -Force

Write-Host ""
Write-Host "Build completed:" -ForegroundColor Green
Write-Host "  $distApp"
Write-Host ""
Write-Host "Usage:"
Write-Host "  1. Double-click $appName.exe"
Write-Host "  2. Click '浏览 Realm' and choose a .realm file"
Write-Host "  3. Click '加载'"
