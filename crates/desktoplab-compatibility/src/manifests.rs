use crate::Channel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeManifest {
    id: String,
    supported_formats: Vec<String>,
    channel: Channel,
}

impl RuntimeManifest {
    #[must_use]
    pub fn new(id: impl Into<String>, supported_formats: &[&str]) -> Self {
        Self {
            id: id.into(),
            supported_formats: supported_formats
                .iter()
                .map(|format| (*format).to_string())
                .collect(),
            channel: Channel::Stable,
        }
    }

    #[must_use]
    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn supports_format(&self, format: &str) -> bool {
        self.supported_formats.iter().any(|known| known == format)
    }

    #[must_use]
    pub fn channel(&self) -> Channel {
        self.channel
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelManifest {
    id: String,
    format: String,
    required_memory_gb: u32,
    channel: Channel,
}

impl ModelManifest {
    #[must_use]
    pub fn new(id: impl Into<String>, format: impl Into<String>, required_memory_gb: u32) -> Self {
        Self {
            id: id.into(),
            format: format.into(),
            required_memory_gb,
            channel: Channel::Stable,
        }
    }

    #[must_use]
    pub fn with_channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    #[must_use]
    pub fn required_memory_gb(&self) -> u32 {
        self.required_memory_gb
    }

    #[must_use]
    pub fn channel(&self) -> Channel {
        self.channel
    }
}
