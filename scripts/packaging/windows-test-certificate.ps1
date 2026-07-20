param(
  [ValidateSet("Create", "Remove")]
  [string]$Action = "Create",
  [string]$Thumbprint
)

$ErrorActionPreference = "Stop"
$subject = "CN=DesktopLab Local Test Signer"
$storePaths = @("Cert:\CurrentUser\My", "Cert:\CurrentUser\TrustedPublisher", "Cert:\LocalMachine\TrustedPeople")

$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw "Windows test certificate management requires an elevated administrator session."
}

if (-not $IsWindows) {
  throw "Windows test certificate management must run on Windows."
}

if ($Action -eq "Remove") {
  if ([string]::IsNullOrWhiteSpace($Thumbprint)) {
    throw "Remove requires -Thumbprint."
  }
  $normalized = $Thumbprint.Replace(" ", "").ToUpperInvariant()
  foreach ($store in $storePaths) {
    $path = Join-Path $store $normalized
    if (Test-Path -LiteralPath $path) {
      Remove-Item -LiteralPath $path -Force
    }
  }
  [ordered]@{ status = "removed"; thumbprint = $normalized; trustScope = "test_host_all_users" } |
    ConvertTo-Json -Compress | Write-Output
  exit 0
}

$existing = Get-ChildItem -Path "Cert:\CurrentUser\My" |
  Where-Object { $_.Subject -eq $subject -and $_.NotAfter -gt (Get-Date).AddDays(1) }
if ($existing) {
  throw "A DesktopLab local test certificate already exists; remove it explicitly before creating another."
}

$certificate = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject $subject `
  -FriendlyName "DesktopLab Local Test Signing" `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -KeyAlgorithm RSA `
  -KeyLength 3072 `
  -HashAlgorithm SHA256 `
  -KeyExportPolicy NonExportable `
  -NotAfter (Get-Date).AddDays(30)

$publicCertificate = Join-Path $env:TEMP "desktoplab-local-test-signing.cer"
try {
  Export-Certificate -Cert $certificate -FilePath $publicCertificate -Force | Out-Null
  Import-Certificate -FilePath $publicCertificate -CertStoreLocation "Cert:\LocalMachine\TrustedPeople" | Out-Null
  Import-Certificate -FilePath $publicCertificate -CertStoreLocation "Cert:\CurrentUser\TrustedPublisher" | Out-Null
} finally {
  Remove-Item -LiteralPath $publicCertificate -Force -ErrorAction SilentlyContinue
}

[ordered]@{
  status = "created"
  thumbprint = $certificate.Thumbprint
  subject = $certificate.Subject
  expiresAt = $certificate.NotAfter.ToUniversalTime().ToString("o")
  privateKeyScope = "current_user_only"
  trustScope = "test_host_all_users"
  trustStores = @("LocalMachine/TrustedPeople", "CurrentUser/TrustedPublisher")
  requiresAdministrator = $true
  privateKeyExportable = $false
  publicTrust = $false
} | ConvertTo-Json -Compress | Write-Output
