param(
  [string]$ArtifactPath = "apps\desktop\src-tauri\target\debug\bundle\nsis\DesktopLab_0.1.0_x64-setup.exe"
)

$ErrorActionPreference = "Stop"

if (-not $IsWindows) {
  throw "Windows host certification must run on Windows."
}

$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $root
$artifact = (Resolve-Path $ArtifactPath).Path
$evidenceDir = Join-Path $root "dist\release"
$smokeLog = Join-Path $evidenceDir "windows-install-smoke.log"
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null

npm.cmd run packaging:verify
if ($LASTEXITCODE -ne 0) {
  throw "Artifact provenance verification failed with exit code $LASTEXITCODE."
}

try {
  $smokeOutput = & (Join-Path $PSScriptRoot "windows-install-smoke.ps1") `
    -ArtifactPath $artifact `
    -RequireValidSignature 2>&1
  $smokeOutput | Tee-Object -FilePath $smokeLog
} catch {
  @($_ | Out-String) | Set-Content -LiteralPath $smokeLog -Encoding utf8
  throw
}

node.exe scripts\packaging\windows-host-evidence.mjs
if ($LASTEXITCODE -ne 0) {
  throw "Windows evidence generation failed with exit code $LASTEXITCODE."
}
