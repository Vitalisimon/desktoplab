use crate::{PluginId, ProviderId, RegistryId, RuntimeId, ToolId};

macro_rules! named_descriptor {
    ($name:ident, $id:ty) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {
            id: $id,
            name: String,
        }

        impl $name {
            #[must_use]
            pub fn new(id: $id, name: impl Into<String>) -> Self {
                Self {
                    id,
                    name: name.into(),
                }
            }

            #[must_use]
            pub fn id(&self) -> &$id {
                &self.id
            }

            #[must_use]
            pub fn name(&self) -> &str {
                &self.name
            }
        }
    };
}

named_descriptor!(RuntimeDescriptor, RuntimeId);
named_descriptor!(ToolDescriptor, ToolId);
named_descriptor!(PluginDescriptor, PluginId);
named_descriptor!(RegistryDescriptor, RegistryId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountMode {
    ApiKeyBilling,
    SubscriptionAccount,
    OauthDevice,
    LocalAppSession,
    CustomEndpoint,
}

impl AccountMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiKeyBilling => "api_key_billing",
            Self::SubscriptionAccount => "subscription_account",
            Self::OauthDevice => "oauth_device",
            Self::LocalAppSession => "local_app_session",
            Self::CustomEndpoint => "custom_endpoint",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAccountDescriptor {
    provider_id: ProviderId,
    mode: AccountMode,
    label: String,
}

impl ProviderAccountDescriptor {
    #[must_use]
    pub fn new(provider_id: ProviderId, mode: AccountMode, label: impl Into<String>) -> Self {
        Self {
            provider_id,
            mode,
            label: label.into(),
        }
    }

    #[must_use]
    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    #[must_use]
    pub fn mode(&self) -> AccountMode {
        self.mode
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityManifest {
    schema: String,
}

impl CompatibilityManifest {
    #[must_use]
    pub fn new(schema: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
        }
    }

    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }
}
