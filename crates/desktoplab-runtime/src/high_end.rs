use crate::RuntimeId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighEndRuntimeFamily {
    Nim,
    TensorRtLlm,
    Vllm,
    LlamaCppServer,
    OpenAiCompatibleLocal,
    CustomLan,
}

impl HighEndRuntimeFamily {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nim => "nim",
            Self::TensorRtLlm => "tensorrt_llm",
            Self::Vllm => "vllm",
            Self::LlamaCppServer => "llamacpp_server",
            Self::OpenAiCompatibleLocal => "openai_compatible_local",
            Self::CustomLan => "custom_lan",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCapabilityState {
    Confirmed,
    Unsupported,
    ProbeRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeLaunchSupport {
    AttachOnly,
    LaunchOrAttach,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSessionOwnership {
    DesktopLab,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInferenceCapabilities {
    tool_calling: RuntimeCapabilityState,
    structured_output: RuntimeCapabilityState,
    streaming: RuntimeCapabilityState,
    batching: RuntimeCapabilityState,
    tensor_parallelism: RuntimeCapabilityState,
    max_context_tokens: Option<u32>,
    max_batch_size: Option<u32>,
    max_tensor_parallel_size: Option<u16>,
    quantization_formats: Vec<String>,
}

impl RuntimeInferenceCapabilities {
    #[must_use]
    pub fn probe_required() -> Self {
        Self {
            tool_calling: RuntimeCapabilityState::ProbeRequired,
            structured_output: RuntimeCapabilityState::ProbeRequired,
            streaming: RuntimeCapabilityState::ProbeRequired,
            batching: RuntimeCapabilityState::ProbeRequired,
            tensor_parallelism: RuntimeCapabilityState::ProbeRequired,
            max_context_tokens: None,
            max_batch_size: None,
            max_tensor_parallel_size: None,
            quantization_formats: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_protocol_support(
        mut self,
        tool_calling: RuntimeCapabilityState,
        structured_output: RuntimeCapabilityState,
        streaming: RuntimeCapabilityState,
    ) -> Self {
        self.tool_calling = tool_calling;
        self.structured_output = structured_output;
        self.streaming = streaming;
        self
    }

    #[must_use]
    pub fn with_context_limit(mut self, max_context_tokens: u32) -> Self {
        self.max_context_tokens = Some(max_context_tokens);
        self
    }

    #[must_use]
    pub fn with_batching(mut self, state: RuntimeCapabilityState, max_batch_size: u32) -> Self {
        self.batching = state;
        self.max_batch_size = Some(max_batch_size);
        self
    }

    #[must_use]
    pub fn with_tensor_parallelism(
        mut self,
        state: RuntimeCapabilityState,
        max_tensor_parallel_size: u16,
    ) -> Self {
        self.tensor_parallelism = state;
        self.max_tensor_parallel_size = Some(max_tensor_parallel_size);
        self
    }

    #[must_use]
    pub fn with_quantization_formats(mut self, formats: &[&str]) -> Self {
        self.quantization_formats = formats.iter().map(ToString::to_string).collect();
        self
    }

    #[must_use]
    pub fn tool_calling(&self) -> RuntimeCapabilityState {
        self.tool_calling
    }

    #[must_use]
    pub fn structured_output(&self) -> RuntimeCapabilityState {
        self.structured_output
    }

    #[must_use]
    pub fn streaming(&self) -> RuntimeCapabilityState {
        self.streaming
    }

    #[must_use]
    pub fn batching(&self) -> RuntimeCapabilityState {
        self.batching
    }

    #[must_use]
    pub fn tensor_parallelism(&self) -> RuntimeCapabilityState {
        self.tensor_parallelism
    }

    #[must_use]
    pub fn max_context_tokens(&self) -> Option<u32> {
        self.max_context_tokens
    }

    #[must_use]
    pub fn max_batch_size(&self) -> Option<u32> {
        self.max_batch_size
    }

    #[must_use]
    pub fn max_tensor_parallel_size(&self) -> Option<u16> {
        self.max_tensor_parallel_size
    }

    #[must_use]
    pub fn quantization_formats(&self) -> &[String] {
        &self.quantization_formats
    }

    #[must_use]
    pub fn has_unverified_fields(&self) -> bool {
        [
            self.tool_calling,
            self.structured_output,
            self.streaming,
            self.batching,
            self.tensor_parallelism,
        ]
        .contains(&RuntimeCapabilityState::ProbeRequired)
            || self.max_context_tokens.is_none()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighEndRuntimeContract {
    runtime_id: RuntimeId,
    family: HighEndRuntimeFamily,
    launch_support: RuntimeLaunchSupport,
    session_ownership: RuntimeSessionOwnership,
    capabilities: RuntimeInferenceCapabilities,
}

impl HighEndRuntimeContract {
    #[must_use]
    pub fn new(
        runtime_id: RuntimeId,
        family: HighEndRuntimeFamily,
        launch_support: RuntimeLaunchSupport,
    ) -> Self {
        Self {
            runtime_id,
            family,
            launch_support,
            session_ownership: RuntimeSessionOwnership::DesktopLab,
            capabilities: RuntimeInferenceCapabilities::probe_required(),
        }
    }

    #[must_use]
    pub fn with_capabilities(mut self, capabilities: RuntimeInferenceCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    #[must_use]
    pub fn family(&self) -> HighEndRuntimeFamily {
        self.family
    }

    #[must_use]
    pub fn launch_support(&self) -> RuntimeLaunchSupport {
        self.launch_support
    }

    #[must_use]
    pub fn session_ownership(&self) -> RuntimeSessionOwnership {
        self.session_ownership
    }

    #[must_use]
    pub fn capabilities(&self) -> &RuntimeInferenceCapabilities {
        &self.capabilities
    }
}

#[must_use]
pub fn high_end_runtime_contracts() -> Vec<HighEndRuntimeContract> {
    [
        (
            "runtime.nim",
            HighEndRuntimeFamily::Nim,
            RuntimeLaunchSupport::LaunchOrAttach,
        ),
        (
            "runtime.tensorrt-llm",
            HighEndRuntimeFamily::TensorRtLlm,
            RuntimeLaunchSupport::LaunchOrAttach,
        ),
        (
            "runtime.vllm",
            HighEndRuntimeFamily::Vllm,
            RuntimeLaunchSupport::LaunchOrAttach,
        ),
        (
            "runtime.llama-cpp-server",
            HighEndRuntimeFamily::LlamaCppServer,
            RuntimeLaunchSupport::LaunchOrAttach,
        ),
        (
            "runtime.openai-compatible-local",
            HighEndRuntimeFamily::OpenAiCompatibleLocal,
            RuntimeLaunchSupport::AttachOnly,
        ),
        (
            "runtime.custom-lan",
            HighEndRuntimeFamily::CustomLan,
            RuntimeLaunchSupport::AttachOnly,
        ),
    ]
    .into_iter()
    .map(|(runtime_id, family, launch_support)| {
        HighEndRuntimeContract::new(RuntimeId::new(runtime_id), family, launch_support)
    })
    .collect()
}
