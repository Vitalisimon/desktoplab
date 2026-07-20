param(
  [Parameter(Mandatory = $true)]
  [string]$ArtifactPath,
  [switch]$RequireValidSignature,
  [switch]$KeepUserData
)

$ErrorActionPreference = "Stop"

if (-not $IsWindows) {
  Write-Output '{"platform":"windows-x64","artifact":"not-run","installState":"not_run","launchState":"not_run","localApiState":"not_run","cleanupState":"not_run"}'
  throw "Windows install smoke must run on Windows."
}

if (-not (Test-Path -LiteralPath $ArtifactPath)) {
  throw "Missing NSIS artifact at $ArtifactPath."
}

if ([System.IO.Path]::GetExtension($ArtifactPath).ToLowerInvariant() -ne ".exe") {
  throw "Windows install smoke expects a NSIS .exe artifact."
}
if ($RequireValidSignature -and (Get-AuthenticodeSignature -LiteralPath $ArtifactPath).Status -ne "Valid") {
  throw "NSIS artifact does not have a valid Authenticode signature."
}

$smokeRoot = Join-Path $env:TEMP ("desktoplab-windows-install-smoke-" + [System.Guid]::NewGuid())
$homeRoot = Join-Path $smokeRoot "home"
$installRoot = Join-Path $smokeRoot "DesktopLab"
$discoveryPath = Join-Path $homeRoot ".config\desktoplab\local-api-discovery.json"
$stdoutLogPath = Join-Path $smokeRoot "desktoplab.stdout.log"
$stderrLogPath = Join-Path $smokeRoot "desktoplab.stderr.log"
$installPreferencePath = "HKCU:\Software\desktoplab\DesktopLab"
$installPreferenceExisted = Test-Path -LiteralPath $installPreferencePath
$previousInstallPreference = $(
  if ($installPreferenceExisted) { (Get-Item -LiteralPath $installPreferencePath).GetValue("") }
  else { $null }
)
$process = $null
$uninstallerPath = $null
$uninstallCompleted = $false
$taskSuffix = [System.Guid]::NewGuid().ToString("N")
$launchTaskName = "DesktopLabInstallSmokeLaunch-$taskSuffix"
$closeTaskName = "DesktopLabInstallSmokeClose-$taskSuffix"
$launchTaskCreated = $false
$closeTaskCreated = $false
$currentSessionId = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
$interactiveSession = Get-Process explorer -ErrorAction SilentlyContinue |
  Where-Object { $_.SessionId -ne $currentSessionId } |
  Select-Object -First 1
$useInteractiveTask = $null -ne $interactiveSession

function Start-InteractiveTask {
  param(
    [Parameter(Mandatory = $true)]
    [string]$TaskName,
    [Parameter(Mandatory = $true)]
    [string]$ScriptPath
  )

  $windowsPowerShell = Join-Path $env:WINDIR "System32\WindowsPowerShell\v1.0\powershell.exe"
  $taskCommand = "`"$windowsPowerShell`" -NoProfile -ExecutionPolicy Bypass -File `"$ScriptPath`""
  $startTime = (Get-Date).AddMinutes(5).ToString("HH:mm")
  & schtasks.exe /Create /TN $TaskName /TR $taskCommand /SC ONCE /ST $startTime /IT /F | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to create interactive task $TaskName."
  }
  & schtasks.exe /Run /TN $TaskName | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to run interactive task $TaskName."
  }
}

New-Item -ItemType Directory -Force -Path $homeRoot, $installRoot | Out-Null

try {
  $installArgs = @(
    "/S",
    "/D=$installRoot"
  )
  $installer = Start-Process -FilePath $ArtifactPath -ArgumentList $installArgs -Wait -PassThru
  if ($installer.ExitCode -ne 0) {
    throw "NSIS install failed with exit code $($installer.ExitCode)."
  }

  $uninstallers = @(Get-ChildItem -LiteralPath $installRoot -Filter "uninstall*.exe" -File)
  if ($uninstallers.Count -ne 1) {
    throw "Expected exactly one NSIS uninstaller, found $($uninstallers.Count)."
  }
  $uninstallerPath = $uninstallers[0].FullName

  $applicationExecutables = @(
    Get-ChildItem -LiteralPath $installRoot -Filter "*.exe" -File |
      Where-Object { $_.Name -notlike "uninstall*.exe" }
  )
  if ($applicationExecutables.Count -ne 1) {
    throw "Expected exactly one installed DesktopLab executable, found $($applicationExecutables.Count)."
  }
  $exePath = $applicationExecutables[0].FullName
  if ($RequireValidSignature -and (Get-AuthenticodeSignature -LiteralPath $exePath).Status -ne "Valid") {
    throw "Installed DesktopLab executable does not have a valid Authenticode signature."
  }

  if ($useInteractiveTask) {
    $launchScriptPath = Join-Path $smokeRoot "launch.ps1"
    @"
`$env:HOME = '$($homeRoot.Replace("'", "''"))'
`$env:USERPROFILE = '$($homeRoot.Replace("'", "''"))'
Set-Location -LiteralPath '$($installRoot.Replace("'", "''"))'
& '$($exePath.Replace("'", "''"))' 1> '$($stdoutLogPath.Replace("'", "''"))' 2> '$($stderrLogPath.Replace("'", "''"))'
"@ | Set-Content -LiteralPath $launchScriptPath -Encoding utf8
    $launchTaskCreated = $true
    Start-InteractiveTask -TaskName $launchTaskName -ScriptPath $launchScriptPath
  } else {
    $process = Start-Process `
      -FilePath $exePath `
      -WorkingDirectory $installRoot `
      -PassThru `
      -RedirectStandardOutput $stdoutLogPath `
      -RedirectStandardError $stderrLogPath `
      -Environment @{ HOME = $homeRoot; USERPROFILE = $homeRoot }
  }

  $baseUrl = $null
  $healthReady = $false
  for ($attempt = 0; $attempt -lt 80; $attempt++) {
    if (Test-Path -LiteralPath $discoveryPath) {
      $discovery = Get-Content -LiteralPath $discoveryPath -Raw | ConvertFrom-Json
      $baseUrl = $discovery.baseUrl
      if ($null -eq $process) {
        $process = Get-Process -Id $discovery.pid -ErrorAction SilentlyContinue
      }
      try {
        Invoke-WebRequest -Uri "$baseUrl/health" -UseBasicParsing -TimeoutSec 2 | Out-Null
        $healthReady = $true
        break
      } catch {
        Start-Sleep -Milliseconds 250
      }
    } else {
      Start-Sleep -Milliseconds 250
    }
  }

  if ([string]::IsNullOrWhiteSpace($baseUrl)) {
    throw "Local API discovery was not created."
  }
  if (-not $healthReady) {
    throw "Local API health did not become ready."
  }
  if ($null -eq $process) {
    throw "DesktopLab process was not available after local API discovery."
  }

  $appStateReady = $false
  for ($attempt = 0; $attempt -lt 20; $attempt++) {
    $appStateStatus = $null
    try {
      Invoke-WebRequest -Uri "$baseUrl/v1/app/state" -UseBasicParsing -TimeoutSec 2 | Out-Null
      $appStateStatus = 200
    } catch {
      if ($null -ne $_.Exception.Response) {
        $appStateStatus = $_.Exception.Response.StatusCode.value__
      }
    }
    if ($appStateStatus -eq 401) {
      $appStateReady = $true
      break
    }
    Start-Sleep -Milliseconds 250
  }
  if (-not $appStateReady) {
    throw "Packaged app state route did not stabilize at 401."
  }
  $discoveryRaw = Get-Content -LiteralPath $discoveryPath -Raw
  if ($discoveryRaw -notmatch '"tokenRedacted":"\[REDACTED_LOCAL_API_TOKEN\]"') {
    throw "Discovery document was not redacted."
  }

  if (-not $process.HasExited) {
    if ($useInteractiveTask) {
      $closeResultPath = Join-Path $smokeRoot "close-result.json"
      $closeScriptPath = Join-Path $smokeRoot "close.ps1"
      @"
`$process = Get-Process -Id $($process.Id) -ErrorAction Stop
`$result = [ordered]@{ mainWindowHandle = `$process.MainWindowHandle; closeRequested = `$process.CloseMainWindow() }
`$process.WaitForExit(10000) | Out-Null
`$result.exited = `$process.HasExited
`$result | ConvertTo-Json -Compress | Set-Content -LiteralPath '$($closeResultPath.Replace("'", "''"))' -Encoding utf8
"@ | Set-Content -LiteralPath $closeScriptPath -Encoding utf8
      $closeTaskCreated = $true
      Start-InteractiveTask -TaskName $closeTaskName -ScriptPath $closeScriptPath
      for ($attempt = 0; $attempt -lt 50 -and -not (Test-Path -LiteralPath $closeResultPath); $attempt++) {
        Start-Sleep -Milliseconds 250
      }
      if (-not (Test-Path -LiteralPath $closeResultPath)) {
        throw "Interactive DesktopLab close task did not return evidence."
      }
      $closeResult = Get-Content -LiteralPath $closeResultPath -Raw | ConvertFrom-Json
      if (-not $closeResult.closeRequested -or -not $closeResult.exited) {
        throw "DesktopLab did not close gracefully from its interactive Windows session."
      }
    } else {
      if (-not $process.CloseMainWindow()) {
        throw "DesktopLab did not expose a main window for graceful shutdown."
      }
      if (-not $process.WaitForExit(10000)) {
        throw "DesktopLab did not exit within 10 seconds of its window close request."
      }
    }
  }

  if (-not $KeepUserData -and (Test-Path -LiteralPath $discoveryPath)) {
    throw "Discovery file should be removed after graceful app shutdown."
  }

  $uninstall = Start-Process -FilePath $uninstallerPath -ArgumentList "/S" -Wait -PassThru
  if ($uninstall.ExitCode -ne 0) {
    throw "NSIS uninstall failed with exit code $($uninstall.ExitCode)."
  }

  $cleanupReady = $false
  for ($attempt = 0; $attempt -lt 120; $attempt++) {
    $registration = Get-ChildItem "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall" -ErrorAction SilentlyContinue |
      ForEach-Object { Get-ItemProperty $_.PSPath } |
      Where-Object {
        $_.DisplayName -eq "DesktopLab" -and
        ([string]$_.InstallLocation).Trim('"') -eq $installRoot
      }
    if (-not (Test-Path -LiteralPath $exePath) -and $null -eq $registration) {
      $cleanupReady = $true
      break
    }
    Start-Sleep -Milliseconds 250
  }
  if (-not $cleanupReady) {
    throw "NSIS uninstall did not remove the application executable and registry entry."
  }
  $uninstallCompleted = $true

  $signatureState = $(if ($RequireValidSignature) { "valid" } else { "not_required" })
  Write-Output ('{"platform":"windows-x64","artifact":"' + [System.IO.Path]::GetFileName($ArtifactPath) + '","signatureState":"' + $signatureState + '","installState":"passed","launchState":"passed","localApiState":"passed","setupState":"auth_required","cleanupState":"passed"}')
  Write-Output "Windows install smoke passed: install, launch, local API health, uninstall."
} finally {
  if ($closeTaskCreated) {
    & schtasks.exe /Delete /TN $closeTaskName /F 2>$null | Out-Null
  }
  if ($launchTaskCreated) {
    & schtasks.exe /Delete /TN $launchTaskName /F 2>$null | Out-Null
  }
  if ($null -ne $process -and -not $process.HasExited) {
    Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    Wait-Process -Id $process.Id -ErrorAction SilentlyContinue
  }
  if (-not $uninstallCompleted -and $uninstallerPath -and (Test-Path -LiteralPath $uninstallerPath)) {
    try {
      $cleanupUninstall = Start-Process -FilePath $uninstallerPath -ArgumentList "/S" -Wait -PassThru
      if ($cleanupUninstall.ExitCode -ne 0) {
        Write-Warning "NSIS cleanup uninstall failed with exit code $($cleanupUninstall.ExitCode)."
      }
    } catch {
      Write-Warning "NSIS cleanup uninstall failed: $($_.Exception.Message)"
    }
  }
  if ($installPreferenceExisted) {
    New-Item -Path $installPreferencePath -Force | Out-Null
    if ($null -eq $previousInstallPreference) {
      Remove-ItemProperty -LiteralPath $installPreferencePath -Name "(default)" -ErrorAction SilentlyContinue
    } else {
      Set-Item -LiteralPath $installPreferencePath -Value $previousInstallPreference
    }
  } elseif (Test-Path -LiteralPath $installPreferencePath) {
    $currentInstallPreference = (Get-Item -LiteralPath $installPreferencePath).GetValue("")
    if ($currentInstallPreference -eq $installRoot) {
      Remove-Item -LiteralPath $installPreferencePath -Recurse -Force
    }
  }
  if (Test-Path -LiteralPath $smokeRoot) {
    Remove-Item -LiteralPath $smokeRoot -Recurse -Force -ErrorAction SilentlyContinue
  }
}
