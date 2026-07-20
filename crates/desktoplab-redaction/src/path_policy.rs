use std::path::Path;

#[must_use]
pub fn is_secret_bearing_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    let file_name = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let segments: Vec<&str> = normalized.split('/').collect();
    segments.iter().any(|segment| {
        *segment == ".env"
            || segment.starts_with(".env.")
            || matches!(
                *segment,
                ".ssh" | ".aws" | ".gnupg" | ".kube" | ".netrc" | ".pypirc" | ".npmrc"
            )
            || segment.contains("credentials")
            || segment.contains("id_rsa")
            || segment.contains("id_dsa")
            || segment.contains("id_ecdsa")
            || segment.contains("id_ed25519")
    }) || matches!(
        file_name,
        "config.json" if normalized.contains(".docker/")
    ) || file_name.starts_with("secrets.")
        || file_name.ends_with(".pem")
        || file_name.ends_with(".key")
        || file_name.ends_with(".p12")
        || file_name.ends_with(".pfx")
        || file_name.ends_with(".jks")
}
