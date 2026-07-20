use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestStatus {
    Stable,
    Beta,
    Deprecated,
    Blocked,
    Revoked,
}

impl ManifestStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Deprecated => "deprecated",
            Self::Blocked => "blocked",
            Self::Revoked => "revoked",
        }
    }

    #[must_use]
    pub fn blocks_recommendation(self) -> bool {
        matches!(self, Self::Blocked | Self::Revoked)
    }
}

impl<'de> Deserialize<'de> for ManifestStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            "deprecated" => Ok(Self::Deprecated),
            "blocked" => Ok(Self::Blocked),
            "revoked" => Ok(Self::Revoked),
            _ => Err(serde::de::Error::custom(format!(
                "unknown manifest status {value}"
            ))),
        }
    }
}

impl Serialize for ManifestStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
