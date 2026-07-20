use std::process::Command;
#[cfg(not(target_os = "windows"))]
use std::{ffi::OsString, path::PathBuf};

pub(crate) fn host_cpu_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    return run_command("/usr/sbin/sysctl", &["-n", "machdep.cpu.brand_string"]);
    #[cfg(target_os = "linux")]
    return std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("model name")
                    .and_then(|entry| entry.split_once(':'))
                    .map(|(_, value)| value.trim().to_string())
            })
        });
    #[cfg(target_os = "windows")]
    return std::env::var("PROCESSOR_IDENTIFIER").ok();
    #[allow(unreachable_code)]
    None
}

pub(crate) fn host_gpu_name() -> Option<String> {
    #[cfg(target_os = "macos")]
    return crate::host_probe_parse::macos_gpu_identity_from_system_report(
        run_command("/usr/sbin/system_profiler", &["SPDisplaysDataType"])
            .as_deref()
            .unwrap_or_default(),
        host_cpu_name().as_deref(),
    );
    #[cfg(target_os = "linux")]
    return run_command("lspci", &[])
        .and_then(|output| crate::host_probe_parse::linux_gpu_identity_from_lspci(&output));
    #[cfg(target_os = "windows")]
    return nvidia_gpu_name().or_else(windows_gpu_fallback);
    #[allow(unreachable_code)]
    None
}

pub(crate) fn host_vram_gb() -> Option<u32> {
    #[cfg(target_os = "linux")]
    return nvidia_vram_gb().or_else(|| {
        run_command("rocm-smi", &["--showmeminfo", "vram"])
            .and_then(|output| crate::host_probe_parse::linux_vram_gb_from_rocm_smi(&output))
    });
    #[cfg(target_os = "windows")]
    return nvidia_vram_gb().or_else(windows_vram_fallback);
    #[allow(unreachable_code)]
    None
}

pub(crate) fn host_ram_gb() -> Option<u32> {
    #[cfg(target_os = "macos")]
    return run_command("/usr/sbin/sysctl", &["-n", "hw.memsize"])
        .and_then(|value| value.parse::<u64>().ok())
        .map(bytes_to_gb);
    #[cfg(target_os = "linux")]
    return std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                line.strip_prefix("MemTotal:")
                    .and_then(|entry| entry.split_whitespace().next())
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| bytes_to_gb(kb * 1024))
            })
        });
    #[cfg(target_os = "windows")]
    return windows_byte_probe(crate::host_probe_parse::windows_ram_probe_powershell_script());
    #[allow(unreachable_code)]
    None
}

pub(crate) fn host_storage_available_gb() -> Option<u32> {
    #[cfg(target_os = "windows")]
    return windows_byte_probe(crate::host_probe_parse::windows_storage_probe_powershell_script());
    #[cfg(not(target_os = "windows"))]
    return storage_probe_path()
        .to_str()
        .and_then(|path| run_command("df", &["-Pk", path]))
        .and_then(|output| crate::host_probe_parse::unix_available_gb_from_df_output(&output));
}

#[cfg(not(target_os = "windows"))]
fn storage_probe_path() -> PathBuf {
    storage_probe_path_from_home(std::env::var_os("HOME"))
}

#[cfg(not(target_os = "windows"))]
fn storage_probe_path_from_home(home: Option<OsString>) -> PathBuf {
    home.filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn nvidia_vram_gb() -> Option<u32> {
    run_command(
        "nvidia-smi",
        &["--query-gpu=memory.total", "--format=csv,noheader,nounits"],
    )
    .and_then(|output| crate::host_probe_parse::linux_vram_gb_from_nvidia_smi(&output))
}

#[cfg(target_os = "windows")]
fn nvidia_gpu_name() -> Option<String> {
    run_command("nvidia-smi", &["--query-gpu=name", "--format=csv,noheader"]).and_then(|output| {
        output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(ToOwned::to_owned)
    })
}

#[cfg(target_os = "windows")]
fn windows_gpu_fallback() -> Option<String> {
    windows_gpu_probe().and_then(|output| {
        crate::host_probe_parse::windows_gpu_identity_from_powershell_output(&output)
    })
}

#[cfg(target_os = "windows")]
fn windows_vram_fallback() -> Option<u32> {
    windows_gpu_probe()
        .and_then(|output| crate::host_probe_parse::windows_vram_gb_from_powershell_output(&output))
}

#[cfg(target_os = "windows")]
fn windows_gpu_probe() -> Option<String> {
    run_command(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            crate::host_probe_parse::windows_gpu_probe_powershell_script(),
        ],
    )
}

#[cfg(target_os = "windows")]
fn windows_byte_probe(script: &str) -> Option<u32> {
    run_command("powershell", &["-NoProfile", "-Command", script]).and_then(|output| {
        crate::host_probe_parse::windows_bytes_gb_from_powershell_output(&output)
    })
}

fn run_command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(not(target_os = "windows"))]
fn bytes_to_gb(bytes: u64) -> u32 {
    bytes.div_ceil(1024 * 1024 * 1024).min(u32::MAX as u64) as u32
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::storage_probe_path_from_home;
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn storage_probe_prefers_user_home_over_packaged_working_directory() {
        assert_eq!(
            storage_probe_path_from_home(Some(OsString::from("/home/simone"))),
            PathBuf::from("/home/simone")
        );
        assert_eq!(storage_probe_path_from_home(None), PathBuf::from("."));
    }
}
