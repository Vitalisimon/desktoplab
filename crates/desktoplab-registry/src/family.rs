use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ManifestFamily {
    Runtime,
    Model,
    Backend,
    Plugin,
    Agent,
    Compatibility,
    Recommendation,
    Advisory,
}

impl ManifestFamily {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Model => "model",
            Self::Backend => "backend",
            Self::Plugin => "plugin",
            Self::Agent => "agent",
            Self::Compatibility => "compatibility",
            Self::Recommendation => "recommendation",
            Self::Advisory => "advisory",
        }
    }

    #[must_use]
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::Runtime => "runtimes",
            Self::Model => "models",
            Self::Backend => "backends",
            Self::Plugin => "plugins",
            Self::Agent => "agents",
            Self::Compatibility => "compatibility",
            Self::Recommendation => "recommendations",
            Self::Advisory => "advisories",
        }
    }
}

impl<'de> Deserialize<'de> for ManifestFamily {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "runtime" => Ok(Self::Runtime),
            "model" => Ok(Self::Model),
            "backend" => Ok(Self::Backend),
            "plugin" => Ok(Self::Plugin),
            "agent" => Ok(Self::Agent),
            "compatibility" => Ok(Self::Compatibility),
            "recommendation" => Ok(Self::Recommendation),
            "advisory" => Ok(Self::Advisory),
            _ => Err(serde::de::Error::custom(format!(
                "unknown manifest family {value}"
            ))),
        }
    }
}

impl Serialize for ManifestFamily {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
