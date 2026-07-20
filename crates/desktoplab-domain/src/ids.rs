macro_rules! domain_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

domain_id!(AgentProfileId);
domain_id!(ExecutionBackendId);
domain_id!(ModelProfileId);
domain_id!(PluginId);
domain_id!(ProviderId);
domain_id!(RegistryId);
domain_id!(RuntimeId);
domain_id!(SessionId);
domain_id!(ToolId);
domain_id!(WorkspaceId);
