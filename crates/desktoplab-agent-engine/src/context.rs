#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentContextBuilder {
    max_bytes: usize,
    chunks: Vec<ContextChunk>,
}

impl AgentContextBuilder {
    #[must_use]
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            chunks: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_workspace_fact(mut self, text: impl Into<String>, provenance: &str) -> Self {
        self.chunks.push(ContextChunk {
            text: text.into(),
            provenance: provenance.to_string(),
            local_only: false,
        });
        self
    }

    #[must_use]
    pub fn with_file(mut self, path: &str, contents: &str, local_only: bool) -> Self {
        self.chunks.push(ContextChunk {
            text: format!("file:{path}\n{contents}"),
            provenance: path.to_string(),
            local_only,
        });
        self
    }

    #[must_use]
    pub fn with_repository_summary(mut self, summary: impl Into<String>, provenance: &str) -> Self {
        self.chunks.push(ContextChunk {
            text: format!("repository:\n{}", summary.into()),
            provenance: provenance.to_string(),
            local_only: false,
        });
        self
    }

    #[must_use]
    pub fn build(self) -> AgentContext {
        let mut text = String::new();
        let mut provenance = Vec::new();

        for chunk in self.chunks.iter().filter(|chunk| !chunk.local_only) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&chunk.text);
            provenance.push(chunk.provenance.clone());
            if text.len() > self.max_bytes {
                text.truncate(self.max_bytes);
                break;
            }
        }

        AgentContext { text, provenance }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ContextChunk {
    text: String,
    provenance: String,
    local_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentContext {
    text: String,
    provenance: Vec<String>,
}

impl AgentContext {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn provenance(&self) -> &[String] {
        &self.provenance
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}
