param(
  [Parameter(Mandatory = $true)]
  [string]$OutputPath
)

$ErrorActionPreference = "Stop"

if (-not $IsWindows) {
  throw "The rustc signing wrapper bootstrap must run on Windows."
}
if ($env:WINDOWS_SIGNING_TRUST_MODE -ne "Test") {
  throw "The rustc signing wrapper is restricted to explicit Windows test signing."
}
if ($env:WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT -notmatch "^[A-Fa-f0-9 ]{40,}$") {
  throw "A valid WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT is required."
}

$rustc = Get-Command rustc.exe -ErrorAction Stop
$sourcePath = Join-Path $PSScriptRoot "windows-rustc-sign-wrapper.rs"
$signingScript = Join-Path $PSScriptRoot "windows-sign.ps1"
$parent = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Path $parent -Force | Out-Null
Remove-Item -LiteralPath $OutputPath -Force -ErrorAction SilentlyContinue

$rustcArguments = @("--edition=2021", $sourcePath, "-o", $OutputPath)
if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER)) {
  $rustcArguments += @("-C", "linker=$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER")
}
& $rustc.Source @rustcArguments
if ($LASTEXITCODE -ne 0) {
  throw "rustc signing wrapper compilation failed with exit code $LASTEXITCODE."
}

& $signingScript -ArtifactPath $OutputPath -TrustMode Test | Out-Null
$signature = Get-AuthenticodeSignature -FilePath $OutputPath
if ($signature.Status -ne "Valid") {
  throw "rustc signing wrapper did not acquire a valid test signature."
}

[ordered]@{
  status = "passed"
  wrapper = $OutputPath
  signatureState = "valid"
  trustMode = "test"
} | ConvertTo-Json -Compress | Write-Output
