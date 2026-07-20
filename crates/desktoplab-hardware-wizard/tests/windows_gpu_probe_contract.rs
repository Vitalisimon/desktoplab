use desktoplab_hardware_wizard::{
    windows_bytes_gb_from_powershell_output, windows_gpu_identity_from_powershell_output,
    windows_gpu_probe_powershell_script, windows_ram_probe_powershell_script,
    windows_storage_probe_powershell_script, windows_vram_gb_from_powershell_output,
};

#[test]
fn windows_gpu_probe_command_strategy_is_explicit() {
    let script = windows_gpu_probe_powershell_script();

    assert!(script.contains("Win32_VideoController"));
    assert!(script.contains("Name"));
    assert!(script.contains("AdapterRAM"));
}

#[test]
fn windows_gpu_identity_is_parsed_from_powershell_output() {
    let output = "Name=NVIDIA GeForce RTX 4070 Laptop GPU\nAdapterRAM=8589934592\n";

    assert_eq!(
        windows_gpu_identity_from_powershell_output(output),
        Some("NVIDIA GeForce RTX 4070 Laptop GPU".to_string())
    );
}

#[test]
fn windows_vram_is_parsed_from_powershell_output() {
    let output = "Name=NVIDIA GeForce RTX 4070 Laptop GPU\nAdapterRAM=8589934592\n";

    assert_eq!(windows_vram_gb_from_powershell_output(output), Some(8));
}

#[test]
fn windows_ram_and_storage_probes_return_parseable_byte_counts() {
    assert!(windows_ram_probe_powershell_script().contains("TotalPhysicalMemory"));
    assert!(windows_storage_probe_powershell_script().contains("AvailableFreeSpace"));
    assert_eq!(
        windows_bytes_gb_from_powershell_output("16340430848\r\n"),
        Some(16)
    );
    assert_eq!(
        windows_bytes_gb_from_powershell_output("360283541504\r\n"),
        Some(336)
    );
}

#[test]
fn windows_format_list_colons_are_parsed() {
    let output = "Name : NVIDIA GeForce RTX 5070 Laptop GPU\r\nAdapterRAM : 8589934592\r\n";

    assert_eq!(
        windows_gpu_identity_from_powershell_output(output),
        Some("NVIDIA GeForce RTX 5070 Laptop GPU".to_string())
    );
    assert_eq!(windows_vram_gb_from_powershell_output(output), Some(8));
}
