#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginHookKind {
    Runtime,
    Provider,
    Tool,
    Backend,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginContractHook {
    kind: PluginHookKind,
    contract_id: String,
}

impl PluginContractHook {
    #[must_use]
    pub fn new(kind: PluginHookKind, contract_id: &str) -> Self {
        Self {
            kind,
            contract_id: contract_id.to_string(),
        }
    }

    #[must_use]
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }

    #[must_use]
    pub fn kind(&self) -> PluginHookKind {
        self.kind
    }

    #[must_use]
    pub fn is_core_contract(&self) -> bool {
        self.contract_id.starts_with("desktoplab.")
    }

    #[must_use]
    pub fn imports_plugin_implementation(&self) -> bool {
        false
    }
}
