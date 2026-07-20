use crate::HardwareProbeAdapter;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HardwareFactSource {
    Detected,
    Declared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierFeatureState {
    Detected,
    NotDetected,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierHardwareClass {
    WorkstationLocal,
    DgxSparkClass,
    DgxStationClass,
    CustomFrontierRig,
    Unclassified,
}

impl FrontierHardwareClass {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkstationLocal => "workstation_local",
            Self::DgxSparkClass => "dgx_spark_class",
            Self::DgxStationClass => "dgx_station_class",
            Self::CustomFrontierRig => "custom_frontier_rig",
            Self::Unclassified => "unclassified",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierHostFacts {
    source: HardwareFactSource,
    observed_at_epoch_seconds: u64,
    gpu_models: Vec<String>,
    accelerator_count: u16,
    accelerator_memory_gb: Option<u32>,
    coherent_memory_gb: Option<u32>,
    cpu_ram_gb: Option<u32>,
    storage_available_gb: Option<u32>,
    cuda_driver_version: Option<String>,
    cuda_runtime_version: Option<String>,
    nvlink: FrontierFeatureState,
    nvswitch: FrontierFeatureState,
    mig: FrontierFeatureState,
}

impl FrontierHostFacts {
    #[must_use]
    pub fn detected(observed_at_epoch_seconds: u64) -> Self {
        Self::new(HardwareFactSource::Detected, observed_at_epoch_seconds)
    }

    #[must_use]
    pub fn declared(observed_at_epoch_seconds: u64) -> Self {
        Self::new(HardwareFactSource::Declared, observed_at_epoch_seconds)
    }

    fn new(source: HardwareFactSource, observed_at_epoch_seconds: u64) -> Self {
        Self {
            source,
            observed_at_epoch_seconds,
            gpu_models: Vec::new(),
            accelerator_count: 0,
            accelerator_memory_gb: None,
            coherent_memory_gb: None,
            cpu_ram_gb: None,
            storage_available_gb: None,
            cuda_driver_version: None,
            cuda_runtime_version: None,
            nvlink: FrontierFeatureState::Unknown,
            nvswitch: FrontierFeatureState::Unknown,
            mig: FrontierFeatureState::Unknown,
        }
    }

    #[must_use]
    pub fn with_accelerators(mut self, gpu_models: &[&str], accelerator_memory_gb: u32) -> Self {
        self.gpu_models = gpu_models.iter().map(ToString::to_string).collect();
        self.accelerator_count = self.gpu_models.len().min(u16::MAX as usize) as u16;
        self.accelerator_memory_gb = Some(accelerator_memory_gb);
        self
    }

    #[must_use]
    pub fn with_memory(mut self, cpu_ram_gb: u32, coherent_memory_gb: Option<u32>) -> Self {
        self.cpu_ram_gb = Some(cpu_ram_gb);
        self.coherent_memory_gb = coherent_memory_gb;
        self
    }

    #[must_use]
    pub fn with_storage_available_gb(mut self, storage_available_gb: u32) -> Self {
        self.storage_available_gb = Some(storage_available_gb);
        self
    }

    #[must_use]
    pub fn with_cuda(mut self, driver_version: &str, runtime_version: &str) -> Self {
        self.cuda_driver_version = Some(driver_version.into());
        self.cuda_runtime_version = Some(runtime_version.into());
        self
    }

    #[must_use]
    pub fn with_topology(
        mut self,
        nvlink: FrontierFeatureState,
        nvswitch: FrontierFeatureState,
        mig: FrontierFeatureState,
    ) -> Self {
        self.nvlink = nvlink;
        self.nvswitch = nvswitch;
        self.mig = mig;
        self
    }

    #[must_use]
    pub fn source(&self) -> HardwareFactSource {
        self.source
    }

    #[must_use]
    pub fn observed_at_epoch_seconds(&self) -> u64 {
        self.observed_at_epoch_seconds
    }

    #[must_use]
    pub fn gpu_models(&self) -> &[String] {
        &self.gpu_models
    }

    #[must_use]
    pub fn accelerator_count(&self) -> u16 {
        self.accelerator_count
    }

    #[must_use]
    pub fn accelerator_memory_gb(&self) -> Option<u32> {
        self.accelerator_memory_gb
    }

    #[must_use]
    pub fn coherent_memory_gb(&self) -> Option<u32> {
        self.coherent_memory_gb
    }

    #[must_use]
    pub fn cpu_ram_gb(&self) -> Option<u32> {
        self.cpu_ram_gb
    }

    #[must_use]
    pub fn storage_available_gb(&self) -> Option<u32> {
        self.storage_available_gb
    }

    #[must_use]
    pub fn cuda_driver_version(&self) -> Option<&str> {
        self.cuda_driver_version.as_deref()
    }

    #[must_use]
    pub fn cuda_runtime_version(&self) -> Option<&str> {
        self.cuda_runtime_version.as_deref()
    }

    #[must_use]
    pub fn nvlink(&self) -> FrontierFeatureState {
        self.nvlink
    }

    #[must_use]
    pub fn nvswitch(&self) -> FrontierFeatureState {
        self.nvswitch
    }

    #[must_use]
    pub fn mig(&self) -> FrontierFeatureState {
        self.mig
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierHardwareAssessment {
    class: FrontierHardwareClass,
    blockers: Vec<String>,
}

impl FrontierHardwareAssessment {
    #[must_use]
    pub fn class(&self) -> FrontierHardwareClass {
        self.class
    }

    #[must_use]
    pub fn is_high_end_candidate(&self) -> bool {
        self.blockers.is_empty()
            && matches!(
                self.class,
                FrontierHardwareClass::DgxSparkClass
                    | FrontierHardwareClass::DgxStationClass
                    | FrontierHardwareClass::CustomFrontierRig
            )
    }

    #[must_use]
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
}

#[derive(Clone, Debug)]
pub struct FrontierHardwareClassifier {
    max_probe_age_seconds: u64,
}

impl FrontierHardwareClassifier {
    #[must_use]
    pub fn new(max_probe_age_seconds: u64) -> Self {
        Self {
            max_probe_age_seconds,
        }
    }

    #[must_use]
    pub fn classify(
        &self,
        facts: &FrontierHostFacts,
        now_epoch_seconds: u64,
    ) -> FrontierHardwareAssessment {
        let mut blockers = Vec::new();
        if facts.source != HardwareFactSource::Detected {
            blockers.push("high-end recommendations require detected hardware facts".into());
        }
        if now_epoch_seconds.saturating_sub(facts.observed_at_epoch_seconds)
            > self.max_probe_age_seconds
        {
            blockers.push("hardware probe is stale".into());
        }
        if facts.gpu_models.is_empty()
            || facts.accelerator_count == 0
            || facts.accelerator_memory_gb.unwrap_or_default() == 0
        {
            blockers.push("accelerator identity or memory evidence is missing".into());
        }
        if facts.cuda_driver_version.is_none() || facts.cuda_runtime_version.is_none() {
            blockers.push("CUDA driver or runtime evidence is missing".into());
        }
        if facts.cpu_ram_gb.is_none() || facts.storage_available_gb.is_none() {
            blockers.push("CPU memory or SSD capacity evidence is missing".into());
        }
        if !blockers.is_empty() {
            return FrontierHardwareAssessment {
                class: FrontierHardwareClass::Unclassified,
                blockers,
            };
        }

        let effective_memory = facts
            .coherent_memory_gb
            .unwrap_or_default()
            .max(facts.accelerator_memory_gb.unwrap_or_default());
        let storage = facts.storage_available_gb.unwrap_or_default();
        let class = if effective_memory >= 700 && storage >= 1_000 {
            FrontierHardwareClass::DgxStationClass
        } else if facts.accelerator_count >= 2 && effective_memory >= 192 && storage >= 1_000 {
            FrontierHardwareClass::CustomFrontierRig
        } else if effective_memory >= 128 && storage >= 500 {
            FrontierHardwareClass::DgxSparkClass
        } else if effective_memory >= 96 {
            FrontierHardwareClass::WorkstationLocal
        } else {
            blockers.push("measured accelerator memory is below the workstation envelope".into());
            FrontierHardwareClass::Unclassified
        };

        FrontierHardwareAssessment { class, blockers }
    }
}

pub trait FrontierProbeSource {
    fn observe(&self) -> FrontierHostFacts;
}

#[derive(Clone, Debug)]
pub struct FrontierHostProbeAdapter<S> {
    source: S,
}

impl<S: FrontierProbeSource> FrontierHostProbeAdapter<S> {
    #[must_use]
    pub fn new(source: S) -> Self {
        Self { source }
    }

    #[must_use]
    pub fn probe(&self) -> FrontierHostFacts {
        self.source.observe()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StdFrontierProbeSource;

impl FrontierProbeSource for StdFrontierProbeSource {
    fn observe(&self) -> FrontierHostFacts {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        let profile = HardwareProbeAdapter::for_current_host().profile();
        let gpu_output = run_nvidia_smi(&["--query-gpu=name", "--format=csv,noheader"]);
        let memory_output =
            run_nvidia_smi(&["--query-gpu=memory.total", "--format=csv,noheader,nounits"]);
        let gpu_models: Vec<&str> = gpu_output
            .as_deref()
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let accelerator_memory_gb = memory_output.as_deref().and_then(sum_memory_gb);
        let driver = run_nvidia_smi(&["--query-gpu=driver_version", "--format=csv,noheader"])
            .and_then(|value| value.lines().next().map(str::trim).map(str::to_string));
        let summary = run_nvidia_smi(&[]).unwrap_or_default();
        let runtime = parse_cuda_runtime_version(&summary);

        let mut facts = FrontierHostFacts::detected(now)
            .with_memory(profile.ram_gb().value(), None)
            .with_storage_available_gb(profile.storage_available_gb().value());
        if let Some(memory_gb) = accelerator_memory_gb {
            facts = facts.with_accelerators(&gpu_models, memory_gb);
        }
        if let (Some(driver), Some(runtime)) = (driver.as_deref(), runtime.as_deref()) {
            facts = facts.with_cuda(driver, runtime);
        }

        facts.with_topology(
            nvlink_state(run_nvidia_smi(&["nvlink", "--status"])),
            nvswitch_state(run_nvidia_smi(&["topo", "-m"])),
            mig_state(run_nvidia_smi(&["-q"])),
        )
    }
}

fn run_nvidia_smi(args: &[&str]) -> Option<String> {
    let output = Command::new("nvidia-smi").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn sum_memory_gb(output: &str) -> Option<u32> {
    let total_mib = output
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .sum::<u64>();
    (total_mib > 0).then(|| total_mib.div_ceil(1024).min(u32::MAX as u64) as u32)
}

fn parse_cuda_runtime_version(output: &str) -> Option<String> {
    let marker = "CUDA Version:";
    let tail = output.split_once(marker)?.1.trim_start();
    tail.split_whitespace().next().map(str::to_string)
}

fn nvlink_state(output: Option<String>) -> FrontierFeatureState {
    match output {
        Some(value)
            if value.lines().any(|line| {
                line.contains("Link") && !line.to_ascii_lowercase().contains("inactive")
            }) =>
        {
            FrontierFeatureState::Detected
        }
        Some(_) => FrontierFeatureState::NotDetected,
        None => FrontierFeatureState::Unknown,
    }
}

fn nvswitch_state(output: Option<String>) -> FrontierFeatureState {
    match output {
        Some(value)
            if value.split_whitespace().any(|token| {
                token
                    .strip_prefix("NV")
                    .is_some_and(|count| count.parse::<u16>().is_ok())
            }) =>
        {
            FrontierFeatureState::Detected
        }
        Some(_) => FrontierFeatureState::NotDetected,
        None => FrontierFeatureState::Unknown,
    }
}

fn mig_state(output: Option<String>) -> FrontierFeatureState {
    match output {
        Some(value)
            if value.lines().any(|line| {
                line.contains("MIG Mode") && line.to_ascii_lowercase().contains("enabled")
            }) =>
        {
            FrontierFeatureState::Detected
        }
        Some(_) => FrontierFeatureState::NotDetected,
        None => FrontierFeatureState::Unknown,
    }
}
