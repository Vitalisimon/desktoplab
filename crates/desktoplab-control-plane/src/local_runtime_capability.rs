use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeAvailability {
    Certified,
    Experimental,
    Planned,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeSetupMode {
    Managed,
    ExternalOnly,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LocalRuntimeCapability {
    availability: RuntimeAvailability,
    setup_mode: RuntimeSetupMode,
    certified_platforms: &'static [&'static str],
    evidence_scope: &'static str,
}

impl LocalRuntimeCapability {
    #[must_use]
    pub(crate) fn for_runtime(runtime_id: &str) -> Self {
        match runtime_id {
            "runtime.ollama" => ollama_capability(),
            "runtime.lm-studio" => Self {
                availability: RuntimeAvailability::Planned,
                setup_mode: RuntimeSetupMode::ExternalOnly,
                certified_platforms: &[],
                evidence_scope: "no_certification_evidence",
            },
            "runtime.mlx-lm" if host_supports_mlx_lm() => Self {
                availability: RuntimeAvailability::Experimental,
                setup_mode: RuntimeSetupMode::None,
                certified_platforms: &[],
                evidence_scope: "no_certification_evidence",
            },
            _ => Self {
                availability: RuntimeAvailability::Unsupported,
                setup_mode: RuntimeSetupMode::None,
                certified_platforms: &[],
                evidence_scope: "unsupported_platform",
            },
        }
    }

    #[must_use]
    pub(crate) fn allows_setup(self) -> bool {
        self.availability == RuntimeAvailability::Certified
            && self.setup_mode == RuntimeSetupMode::Managed
    }

    #[must_use]
    pub(crate) fn blocked_reason(self) -> &'static str {
        match self.availability {
            RuntimeAvailability::Certified => "runtime setup is available",
            RuntimeAvailability::Experimental => "Preview; not certified",
            RuntimeAvailability::Planned => "Planned runtime",
            RuntimeAvailability::Unsupported => "Not available on this computer",
        }
    }

    #[must_use]
    pub(crate) fn to_json(self) -> Value {
        json!({
            "availability":availability_value(self.availability),
            "setupMode":setup_mode_value(self.setup_mode),
            "verification":if self.availability == RuntimeAvailability::Certified {
                "unverified"
            } else {
                "not_applicable"
            },
            "certifiedPlatforms":self.certified_platforms,
            "evidenceScope":self.evidence_scope
        })
    }
}

fn ollama_capability() -> LocalRuntimeCapability {
    const CERTIFIED_PLATFORMS: &[&str] = &["macos-aarch64", "linux-x64"];
    if CERTIFIED_PLATFORMS.contains(&current_platform()) {
        return LocalRuntimeCapability {
            availability: RuntimeAvailability::Certified,
            setup_mode: RuntimeSetupMode::Managed,
            certified_platforms: CERTIFIED_PLATFORMS,
            evidence_scope: "exact_candidate_required",
        };
    }
    LocalRuntimeCapability {
        availability: RuntimeAvailability::Experimental,
        setup_mode: RuntimeSetupMode::None,
        certified_platforms: CERTIFIED_PLATFORMS,
        evidence_scope: "platform_certification_required",
    }
}

fn availability_value(availability: RuntimeAvailability) -> &'static str {
    match availability {
        RuntimeAvailability::Certified => "certified",
        RuntimeAvailability::Experimental => "experimental",
        RuntimeAvailability::Planned => "planned",
        RuntimeAvailability::Unsupported => "unsupported",
    }
}

fn setup_mode_value(mode: RuntimeSetupMode) -> &'static str {
    match mode {
        RuntimeSetupMode::Managed => "managed",
        RuntimeSetupMode::ExternalOnly => "external_only",
        RuntimeSetupMode::None => "none",
    }
}

fn current_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "macos-aarch64",
        ("linux", "x86_64") => "linux-x64",
        ("windows", "x86_64") => "windows-x64",
        _ => "unsupported",
    }
}

fn host_supports_mlx_lm() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}
