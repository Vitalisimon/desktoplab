use desktoplab_domain::{
    AccountMode, AgentProfile, AgentProfileId, ApprovalPolicy, BackendCapability,
    CompatibilityManifest, ExecutionBackend, ExecutionBackendId, ExecutionBackendKind,
    ExecutionRoute, HardwareProfile, MemoryScope, ModelProfile, ModelProfileId, PluginDescriptor,
    PluginId, Policy, Provider, ProviderAccountDescriptor, ProviderId, ProviderKind,
    RegistryDescriptor, RegistryId, RuntimeDescriptor, RuntimeId, Session, SessionId, SessionOwner,
    ToolDescriptor, ToolId, Workspace, WorkspaceId,
};

#[test]
fn provider_and_execution_backend_are_separate_domain_concepts() {
    let provider = Provider::new(
        ProviderId::new("provider.openai"),
        "OpenAI",
        ProviderKind::CloudAccount,
    );
    let backend = ExecutionBackend::new(
        ExecutionBackendId::new("backend.ollama.local"),
        "Local Ollama",
        ExecutionBackendKind::LocalRuntime,
        None,
    )
    .with_capability(BackendCapability::new("llm.chat"));

    assert_eq!(provider.id().as_str(), "provider.openai");
    assert_eq!(backend.id().as_str(), "backend.ollama.local");
    assert_ne!(provider.id().as_str(), backend.id().as_str());
    assert_eq!(backend.provider_id(), None);
    assert!(backend.supports("llm.chat"));
}

#[test]
fn execution_backend_may_reference_provider_without_becoming_provider() {
    let provider_id = ProviderId::new("provider.anthropic");
    let backend = ExecutionBackend::new(
        ExecutionBackendId::new("backend.claude.agent-sdk"),
        "Claude Agent SDK",
        ExecutionBackendKind::ExternalAgent,
        Some(provider_id.clone()),
    );

    assert_eq!(backend.provider_id(), Some(&provider_id));
    assert_eq!(backend.kind(), ExecutionBackendKind::ExternalAgent);
}

#[test]
fn provider_account_modes_are_distinct_domain_contracts() {
    let modes = [
        AccountMode::ApiKeyBilling,
        AccountMode::SubscriptionAccount,
        AccountMode::OauthDevice,
        AccountMode::LocalAppSession,
        AccountMode::CustomEndpoint,
    ];

    assert_eq!(
        modes.map(AccountMode::as_str),
        [
            "api_key_billing",
            "subscription_account",
            "oauth_device",
            "local_app_session",
            "custom_endpoint",
        ]
    );
}

#[test]
fn execution_backend_can_reference_account_mode_without_becoming_provider() {
    let provider_id = ProviderId::new("provider.openai");
    let account = ProviderAccountDescriptor::new(
        provider_id.clone(),
        AccountMode::SubscriptionAccount,
        "ChatGPT Team",
    );
    let backend = ExecutionBackend::new(
        ExecutionBackendId::new("backend.codex.app-server"),
        "Codex App Server",
        ExecutionBackendKind::ExternalAgent,
        Some(provider_id.clone()),
    )
    .with_account_mode(account.mode());

    assert_eq!(account.provider_id(), &provider_id);
    assert_eq!(account.mode(), AccountMode::SubscriptionAccount);
    assert_eq!(account.label(), "ChatGPT Team");
    assert_eq!(backend.provider_id(), Some(&provider_id));
    assert_eq!(
        backend.account_mode(),
        Some(AccountMode::SubscriptionAccount)
    );
    assert_eq!(backend.kind(), ExecutionBackendKind::ExternalAgent);
}

#[test]
fn desktoplab_owns_session_even_when_backend_executes() {
    let workspace = Workspace::new(WorkspaceId::new("workspace.desktoplab"));
    let backend_id = ExecutionBackendId::new("backend.codex.app-server");
    let session = Session::new(
        SessionId::new("session.001"),
        workspace.id().clone(),
        backend_id.clone(),
    );

    assert_eq!(session.owner(), SessionOwner::DesktopLab);
    assert_eq!(session.execution_backend_id(), &backend_id);
    assert_eq!(session.workspace_id(), workspace.id());
}

#[test]
fn backend_capabilities_are_data_not_vendor_branches() {
    let backend = ExecutionBackend::new(
        ExecutionBackendId::new("backend.custom.openai-compatible"),
        "Custom OpenAI-Compatible Endpoint",
        ExecutionBackendKind::RemoteRunner,
        None,
    )
    .with_capability(BackendCapability::new("llm.chat"))
    .with_capability(BackendCapability::new("tools.filesystem.read"))
    .with_capability(BackendCapability::new("agent.events.stream"));

    let route = ExecutionRoute::new(
        backend.id().clone(),
        vec![BackendCapability::new("llm.chat")],
    );

    assert!(backend.supports_all(route.required_capabilities()));
    assert!(!backend.supports("git.push"));
}

#[test]
fn core_domain_vocabulary_has_first_class_contracts() {
    let runtime = RuntimeDescriptor::new(RuntimeId::new("runtime.ollama"), "Ollama");
    let model = ModelProfile::new(ModelProfileId::new("model.qwen3"), "Qwen3");
    let hardware = HardwareProfile::new("macos", "aarch64");
    let agent = AgentProfile::new(
        AgentProfileId::new("agent.desktoplab.local"),
        "DesktopLab Local Agent",
    );
    let tool = ToolDescriptor::new(ToolId::new("tool.filesystem.read"), "Filesystem Read");
    let plugin = PluginDescriptor::new(PluginId::new("plugin.acp"), "ACP Backend Plugin");
    let registry = RegistryDescriptor::new(
        RegistryId::new("registry.desktoplab"),
        "DesktopLab Registry",
    );
    let policy = Policy::new(ApprovalPolicy::Conservative);
    let memory = MemoryScope::Workspace;
    let compatibility = CompatibilityManifest::new("registry.desktoplab.dev/v1");

    assert_eq!(runtime.name(), "Ollama");
    assert_eq!(model.name(), "Qwen3");
    assert_eq!(hardware.operating_system(), "macos");
    assert_eq!(agent.name(), "DesktopLab Local Agent");
    assert_eq!(tool.name(), "Filesystem Read");
    assert_eq!(plugin.name(), "ACP Backend Plugin");
    assert_eq!(registry.name(), "DesktopLab Registry");
    assert_eq!(policy.approval_policy(), ApprovalPolicy::Conservative);
    assert_eq!(memory, MemoryScope::Workspace);
    assert_eq!(compatibility.schema(), "registry.desktoplab.dev/v1");
}
