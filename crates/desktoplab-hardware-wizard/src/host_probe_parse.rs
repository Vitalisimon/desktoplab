#[must_use]
pub fn macos_gpu_identity_from_system_report(
    report: &str,
    cpu_name: Option<&str>,
) -> Option<String> {
    report
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("Chipset Model:")
                .map(str::trim)
                .filter(|value| value.contains("Apple"))
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            cpu_name
                .map(str::trim)
                .filter(|value| value.contains("Apple M"))
                .map(ToOwned::to_owned)
        })
}

#[must_use]
pub fn linux_gpu_identity_from_lspci(lspci: &str) -> Option<String> {
    let candidates = lspci
        .lines()
        .filter(|line| {
            let normalized = line.to_ascii_lowercase();
            normalized.contains("vga")
                || normalized.contains("3d controller")
                || normalized.contains("display controller")
        })
        .filter_map(|line| {
            line.rsplit_once(": ")
                .map(|(_, value)| value.trim().to_string())
        })
        .collect::<Vec<_>>();

    candidates
        .iter()
        .find(|value| {
            let normalized = value.to_ascii_lowercase();
            normalized.contains("nvidia")
                || normalized.contains("amd")
                || normalized.contains("radeon")
        })
        .cloned()
        .or_else(|| candidates.first().cloned())
}

#[must_use]
pub fn linux_vram_gb_from_nvidia_smi(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let numeric = line
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())?;
        Some(((numeric + 1023) / 1024).min(u32::MAX as u64) as u32)
    })
}

#[must_use]
pub fn linux_vram_gb_from_rocm_smi(output: &str) -> Option<u32> {
    output.lines().find_map(|line| {
        let (_, value) = line.split_once("VRAM Total Memory (B):")?;
        let bytes = value.trim().parse::<u64>().ok()?;
        Some(bytes.div_ceil(1024 * 1024 * 1024).min(u32::MAX as u64) as u32)
    })
}

#[must_use]
pub fn unix_available_gb_from_df_output(output: &str) -> Option<u32> {
    let available_kb = output
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .last()?
        .split_whitespace()
        .nth(3)?
        .parse::<u64>()
        .ok()?;
    Some(available_kb.div_ceil(1024 * 1024).min(u32::MAX as u64) as u32)
}

#[must_use]
pub fn windows_gpu_probe_powershell_script() -> &'static str {
    "Get-CimInstance Win32_VideoController | Select-Object -First 1 Name,AdapterRAM | Format-List"
}

#[must_use]
pub fn windows_ram_probe_powershell_script() -> &'static str {
    "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory"
}

#[must_use]
pub fn windows_storage_probe_powershell_script() -> &'static str {
    "[System.IO.DriveInfo]::new($env:SystemDrive).AvailableFreeSpace"
}

#[must_use]
pub fn windows_bytes_gb_from_powershell_output(output: &str) -> Option<u32> {
    let bytes = output.trim().parse::<u64>().ok()?;
    Some(bytes.div_ceil(1024 * 1024 * 1024).min(u32::MAX as u64) as u32)
}

#[must_use]
pub fn windows_gpu_identity_from_powershell_output(output: &str) -> Option<String> {
    key_value(output, "Name").map(ToOwned::to_owned)
}

#[must_use]
pub fn windows_vram_gb_from_powershell_output(output: &str) -> Option<u32> {
    let bytes = key_value(output, "AdapterRAM")?.parse::<u64>().ok()?;
    Some(bytes.div_ceil(1024 * 1024 * 1024).min(u32::MAX as u64) as u32)
}

fn key_value<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=').or_else(|| line.split_once(':'))?;
        (candidate.trim() == key)
            .then(|| value.trim())
            .filter(|value| !value.is_empty())
    })
}
