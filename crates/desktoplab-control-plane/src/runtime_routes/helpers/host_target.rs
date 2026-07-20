pub(in crate::runtime_routes) fn host_target() -> String {
    normalized_host_target(std::env::consts::OS, std::env::consts::ARCH)
}

pub(in crate::runtime_routes) fn host_supports_mlx_lm() -> bool {
    matches!(host_target().as_str(), "macos-aarch64" | "darwin-arm64")
}

fn normalized_host_target(os: &str, arch: &str) -> String {
    let normalized_arch = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    let normalized_os = match os {
        "macos" => "darwin",
        other => other,
    };
    format!("{normalized_os}-{normalized_arch}")
}

#[cfg(test)]
mod tests {
    use super::normalized_host_target;

    #[test]
    fn normalizes_linux_x86_64_to_runtime_target_vocabulary() {
        assert_eq!(normalized_host_target("linux", "x86_64"), "linux-x64");
    }

    #[test]
    fn normalizes_macos_aarch64_to_runtime_target_vocabulary() {
        assert_eq!(normalized_host_target("macos", "aarch64"), "darwin-arm64");
    }

    #[test]
    fn normalizes_windows_x86_64_to_runtime_target_vocabulary() {
        assert_eq!(normalized_host_target("windows", "x86_64"), "windows-x64");
    }
}
