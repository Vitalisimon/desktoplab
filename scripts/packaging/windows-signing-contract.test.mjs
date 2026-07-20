import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { windowsAuthenticodeState } from "./windows-authenticode-state.mjs";

const signing = readFileSync("scripts/packaging/windows-sign.ps1", "utf8");
const testCertificate = readFileSync("scripts/packaging/windows-test-certificate.ps1", "utf8");
const rustcBootstrap = readFileSync("scripts/packaging/windows-rustc-signing-bootstrap.ps1", "utf8");
const rustcWrapper = readFileSync("scripts/packaging/windows-rustc-sign-wrapper.rs", "utf8");
const build = readFileSync("scripts/packaging/build-dev.sh", "utf8");
const parity = readFileSync("scripts/product/cross-platform-agent-parity.mjs", "utf8");
const metadata = readFileSync("scripts/packaging/prepare-build-metadata.mjs", "utf8");
const artifacts = readFileSync("scripts/packaging/record-artifacts.mjs", "utf8");
const authenticode = readFileSync("scripts/packaging/windows-authenticode-state.mjs", "utf8");
const smoke = readFileSync("scripts/packaging/windows-install-smoke.ps1", "utf8");
const hostCertify = readFileSync("scripts/packaging/windows-host-certify.ps1", "utf8");
const storageManifest = readFileSync("crates/desktoplab-storage/Cargo.toml", "utf8");
const desktopLockfile = readFileSync("apps/desktop/src-tauri/Cargo.lock", "utf8");
const windowsConfig = readFileSync("apps/desktop/src-tauri/tauri.windows.conf.json", "utf8");
const tauriSchema = readFileSync("node_modules/@tauri-apps/cli/config.schema.json", "utf8");

test("test signing is store-backed, self-signed-only, and explicitly non-public", () => {
  assert.match(signing, /TrustMode -eq "Test"/);
  assert.match(signing, /WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT/);
  assert.match(signing, /Test mode accepts only an explicitly self-signed certificate/);
  assert.match(signing, /\$enhancedKeyUsageOids -notcontains \$codeSigningOid/);
  assert.match(signing, /\$\_\.ObjectId -is \[string\]/);
  assert.match(signing, /publicTrust = \$TrustMode -eq "Public"/);
  assert.doesNotMatch(testCertificate, /Export-PfxCertificate/);
  assert.match(testCertificate, /KeyExportPolicy NonExportable/);
  assert.match(testCertificate, /privateKeyScope = "current_user_only"/);
  assert.match(testCertificate, /trustScope = "test_host_all_users"/);
  assert.match(testCertificate, /requiresAdministrator = \$true/);
  assert.match(testCertificate, /publicTrust = \$false/);
});

test("public signing rejects self-signed input and requires SHA-256 timestamping", () => {
  assert.match(signing, /Public mode refuses self-signed certificates/);
  assert.match(signing, /Public signing requires WINDOWS_SIGNING_TIMESTAMP_URL/);
  assert.match(signing, /"\/fd", "SHA256"/);
  assert.match(signing, /"\/tr", \$timestampUrl, "\/td", "SHA256"/);
  assert.match(signing, /verify \/pa \/v/);
});

test("test certificate cleanup removes all current-user trust copies", () => {
  assert.match(testCertificate, /ValidateSet\("Create", "Remove"\)/);
  assert.match(testCertificate, /Cert:\\CurrentUser\\My/);
  assert.match(testCertificate, /Cert:\\CurrentUser\\TrustedPublisher/);
  assert.match(testCertificate, /Cert:\\LocalMachine\\TrustedPeople/);
  assert.doesNotMatch(testCertificate, /Cert:\\CurrentUser\\Root/);
  assert.match(testCertificate, /WindowsBuiltInRole\]::Administrator/);
  assert.match(testCertificate, /Remove-Item -LiteralPath \$path -Force/);
  assert.match(testCertificate, /trustScope = "test_host_all_users"/);
});

test("Smart App Control test builds sign rustc outputs without weakening host policy", () => {
  assert.match(rustcBootstrap, /WINDOWS_SIGNING_TRUST_MODE -ne "Test"/);
  assert.match(rustcBootstrap, /windows-rustc-sign-wrapper\.rs/);
  assert.match(rustcBootstrap, /windows-sign\.ps1/);
  assert.match(rustcBootstrap, /Get-AuthenticodeSignature/);
  assert.match(rustcBootstrap, /CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER/);
  assert.match(rustcBootstrap, /linker=\$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER/);
  assert.match(rustcWrapper, /Command::new\(rustc\)/);
  assert.match(rustcWrapper, /Command::new\("signtool\.exe"\)/);
  assert.match(rustcWrapper, /portable_executable_outputs/);
  assert.match(rustcWrapper, /"proc-macro" \| "dylib" \| "cdylib"/);
  assert.match(rustcWrapper, /"exe"\) \|\| extension\.eq_ignore_ascii_case\("dll"\)/);
  assert.match(parity, /prepareSignedWindowsRustcWrapper/);
  assert.match(build, /configure_windows_test_rustc_signing/);
  assert.match(build, /RUSTC_WRAPPER is already configured/);
  assert.doesNotMatch(rustcBootstrap, /VerifiedAndReputablePolicyState|CiTool|Set-ItemProperty/);
});

test("Windows signing scripts stay reviewable", () => {
  assert.ok(signing.split(/\r?\n/).length <= 145);
  assert.ok(testCertificate.split(/\r?\n/).length <= 100);
  assert.ok(rustcBootstrap.split(/\r?\n/).length <= 70);
  assert.ok(rustcWrapper.split(/\r?\n/).length <= 190);
});

test("signed Windows packaging passes the certificate to Tauri", () => {
  assert.match(build, /windows-sign\.ps1/);
  assert.match(build, /-Preflight/);
  assert.match(build, /where\.exe link\.exe/);
  assert.match(build, /CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER/);
  assert.match(build, /Hostx64\\\\x64\\\\link\.exe/);
  assert.match(metadata, /certificateThumbprint/);
  assert.match(metadata, /digestAlgorithm: "sha256"/);
  assert.match(metadata, /WINDOWS_SIGNING_TIMESTAMP_URL/);
  assert.match(metadata, /signingTrustMode/);
  assert.match(windowsConfig, /"targets": \["nsis"\]/);
  assert.match(tauriSchema, /tauri\.windows\.conf\.json/);
  assert.match(tauriSchema, /gets merged with the main configuration object/);
  assert.match(build, /Locally signed dev packaging artifacts recorded/);
});

test("Windows packaging does not depend on a system SQLite installation", () => {
  assert.match(storageManifest, /rusqlite\s*=\s*\{[^}]*features\s*=\s*\["bundled"\]/);
  assert.match(
    desktopLockfile,
    /name = "libsqlite3-sys"[\s\S]*?dependencies = \[[\s\S]*?"cc",[\s\S]*?\]/,
  );
});

test("artifact recording and install smoke verify Windows Authenticode", () => {
  assert.match(artifacts, /windowsAuthenticodeState/);
  assert.match(authenticode, /signtool\.exe/);
  assert.match(authenticode, /\["verify", "\/pa", artifactPath\]/);
  assert.match(authenticode, /Get-AuthenticodeSignature/);
  assert.match(authenticode, /return "signed"/);
  assert.match(smoke, /RequireValidSignature/);
  assert.match(smoke, /NSIS artifact does not have a valid Authenticode signature/);
  assert.match(smoke, /\$applicationExecutables = @\(/);
  assert.match(smoke, /\$applicationExecutables\.Count -ne 1/);
  assert.match(smoke, /Installed DesktopLab executable does not have a valid Authenticode signature/);
  assert.match(smoke, /RedirectStandardOutput \$stdoutLogPath/);
  assert.match(smoke, /RedirectStandardError \$stderrLogPath/);
  assert.match(smoke, /\$healthReady = \$false/);
  assert.match(smoke, /if \(-not \$healthReady\)/);
  assert.match(smoke, /\$appStateReady = \$false/);
  assert.match(smoke, /if \(-not \$appStateReady\)/);
  assert.match(smoke, /\$process\.CloseMainWindow\(\)/);
  assert.match(smoke, /Get-Process explorer/);
  assert.match(smoke, /schtasks\.exe \/Create/);
  assert.match(smoke, /schtasks\.exe \/Run/);
  assert.match(smoke, /DesktopLabInstallSmokeLaunch-/);
  assert.match(smoke, /DesktopLabInstallSmokeClose-/);
  assert.match(smoke, /DesktopLab did not close gracefully from its interactive Windows session/);
  assert.match(smoke, /schtasks\.exe \/Delete/);
  assert.match(smoke, /\$process\.WaitForExit\(10000\)/);
  assert.match(smoke, /\$cleanupReady = \$false/);
  assert.match(smoke, /\$attempt -lt 120/);
  assert.match(smoke, /Test-Path -LiteralPath \$exePath/);
  assert.match(smoke, /HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall/);
  assert.match(smoke, /HKCU:\\Software\\desktoplab\\DesktopLab/);
  assert.match(smoke, /if \(\$null -eq \$previousInstallPreference\)/);
  assert.match(smoke, /Remove-ItemProperty -LiteralPath \$installPreferencePath -Name "\(default\)"/);
  assert.match(smoke, /Set-Item -LiteralPath \$installPreferencePath -Value \$previousInstallPreference/);
  assert.doesNotMatch(smoke, /\.SetValue\(/);
  assert.match(smoke, /\$previousInstallPreference/);
  assert.match(smoke, /\$currentInstallPreference -eq \$installRoot/);
  assert.match(smoke, /if \(-not \$cleanupReady\)/);
  assert.match(smoke, /if \(-not \$uninstallCompleted -and \$uninstallerPath/);
  assert.match(smoke, /"signatureState":"' \+ \$signatureState/);
});

test("Windows host certification records signed install smoke evidence", () => {
  assert.match(hostCertify, /windows-install-smoke\.ps1/);
  assert.match(hostCertify, /RequireValidSignature/);
  assert.match(hostCertify, /windows-install-smoke\.log/);
  assert.match(hostCertify, /windows-host-evidence\.mjs/);
});

test("artifact recording retries transient Authenticode visibility", () => {
  const statuses = ["NotSigned", "NotSigned", "Valid"];
  let waits = 0;
  const state = windowsAuthenticodeState("candidate.exe", {
    attempts: 3,
    readStatus: () => statuses.shift(),
    wait: () => { waits += 1; },
  });
  assert.equal(state, "signed");
  assert.equal(waits, 2);
});

test("artifact recording remains fail-closed when Authenticode never validates", () => {
  const state = windowsAuthenticodeState("candidate.exe", {
    attempts: 2,
    readStatus: () => "NotSigned",
    wait: () => {},
  });
  assert.equal(state, "unsigned_dev");
});
