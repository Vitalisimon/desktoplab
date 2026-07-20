param(
  [string]$ArtifactPath,
  [ValidateSet("Test", "Public")]
  [string]$TrustMode = "Public",
  [switch]$Preflight,
  [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$certificatePath = $env:WINDOWS_SIGNING_CERTIFICATE_PATH
$certificatePassword = $env:WINDOWS_SIGNING_CERTIFICATE_PASSWORD
$certificateThumbprint = $env:WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT
$timestampUrl = $env:WINDOWS_SIGNING_TIMESTAMP_URL

if ($DryRun) {
  Write-Output "DryRun: Windows signing boundary OK."
  Write-Output "DryRun: Test mode accepts only a CurrentUser certificate-store thumbprint and never establishes public trust."
  Write-Output "DryRun: Public mode rejects self-signed store certificates and requires an RFC 3161 timestamp."
  exit 0
}

if (-not $IsWindows) {
  throw "Windows signing must run on Windows."
}
$usesPfx = -not [string]::IsNullOrWhiteSpace($certificatePath)
$usesStore = -not [string]::IsNullOrWhiteSpace($certificateThumbprint)
if ($usesPfx -eq $usesStore) {
  throw "Configure exactly one Windows signing source: certificate path or CurrentUser store thumbprint."
}
if ($TrustMode -eq "Test" -and -not $usesStore) {
  throw "Test signing requires WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT."
}
if ($TrustMode -eq "Public" -and [string]::IsNullOrWhiteSpace($timestampUrl)) {
  throw "Public signing requires WINDOWS_SIGNING_TIMESTAMP_URL."
}

$certificate = $null
if ($usesStore) {
  $normalizedThumbprint = $certificateThumbprint.Replace(" ", "").ToUpperInvariant()
  $certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$normalizedThumbprint" -ErrorAction SilentlyContinue
  if ($null -eq $certificate -or -not $certificate.HasPrivateKey) {
    throw "Signing certificate with private key was not found in Cert:\CurrentUser\My."
  }
  $codeSigningOid = "1.3.6.1.5.5.7.3.3"
  $enhancedKeyUsageOids = @($certificate.EnhancedKeyUsageList | ForEach-Object {
    if ($_.ObjectId -is [string]) { $_.ObjectId } else { $_.ObjectId.Value }
  })
  if ($enhancedKeyUsageOids -notcontains $codeSigningOid) {
    throw "Selected certificate is not valid for code signing."
  }
  $selfSigned = $certificate.Subject -eq $certificate.Issuer
  if ($TrustMode -eq "Test" -and -not $selfSigned) {
    throw "Test mode accepts only an explicitly self-signed certificate."
  }
  if ($TrustMode -eq "Public" -and $selfSigned) {
    throw "Public mode refuses self-signed certificates."
  }
} else {
  if (-not (Test-Path -LiteralPath $certificatePath)) {
    throw "WINDOWS_SIGNING_CERTIFICATE_PATH does not exist."
  }
  if ([string]::IsNullOrWhiteSpace($certificatePassword)) {
    throw "WINDOWS_SIGNING_CERTIFICATE_PASSWORD is required for PFX signing."
  }
}

$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if ($null -eq $signtool) {
  throw "signtool.exe is required for Windows signing."
}

if ($Preflight) {
  [ordered]@{
    status = "passed"
    trustMode = $TrustMode.ToLowerInvariant()
    publicTrust = $TrustMode -eq "Public"
    certificateSource = $(if ($usesStore) { "current_user_store" } else { "pfx" })
    timestampConfigured = -not [string]::IsNullOrWhiteSpace($timestampUrl)
  } | ConvertTo-Json -Compress | Write-Output
  exit 0
}
if ([string]::IsNullOrWhiteSpace($ArtifactPath) -or -not (Test-Path -LiteralPath $ArtifactPath)) {
  throw "Missing -ArtifactPath for Windows artifact; refusing to sign."
}

$arguments = @("sign", "/fd", "SHA256")
if ($usesStore) {
  $arguments += @("/sha1", $normalizedThumbprint, "/s", "My")
} else {
  $arguments += @("/f", $certificatePath, "/p", $certificatePassword)
}
if (-not [string]::IsNullOrWhiteSpace($timestampUrl)) {
  $arguments += @("/tr", $timestampUrl, "/td", "SHA256")
}
$arguments += $ArtifactPath

& $signtool.Source @arguments
if ($LASTEXITCODE -ne 0) {
  throw "signtool failed with exit code $LASTEXITCODE."
}

& $signtool.Source verify /pa /v $ArtifactPath
if ($LASTEXITCODE -ne 0) {
  throw "signtool verification failed with exit code $LASTEXITCODE."
}

[ordered]@{
  status = "passed"
  trustMode = $TrustMode.ToLowerInvariant()
  publicTrust = $TrustMode -eq "Public"
  artifact = [System.IO.Path]::GetFileName($ArtifactPath)
  certificateSource = $(if ($usesStore) { "current_user_store" } else { "pfx" })
  timestamped = -not [string]::IsNullOrWhiteSpace($timestampUrl)
} | ConvertTo-Json -Compress | Write-Output
