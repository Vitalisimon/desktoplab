use crate::{IndexedCodeDocument, RepoCodeIndexSnapshot, RepoIndexFreshnessReport};
use desktoplab_redaction::redact_repository_context;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingBackendLocality {
    Local,
    External,
}

pub trait LocalEmbeddingBackend {
    fn locality(&self) -> EmbeddingBackendLocality;
    fn embed(&self, text: &str) -> Result<Vec<f32>, String>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetrievalStrategy {
    Lexical,
    Symbol,
    Dependency,
    Recency,
    LocalEmbedding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetrievalProvenance {
    path: String,
    content_hash: String,
    index_generation: String,
    start_line: usize,
    end_line: usize,
}

impl RetrievalProvenance {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    #[must_use]
    pub fn index_generation(&self) -> &str {
        &self.index_generation
    }

    #[must_use]
    pub fn start_line(&self) -> usize {
        self.start_line
    }

    #[must_use]
    pub fn end_line(&self) -> usize {
        self.end_line
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetrievedContextItem {
    snippet: String,
    score: f32,
    strategies: Vec<RetrievalStrategy>,
    provenance: RetrievalProvenance,
    redacted: bool,
}

impl RetrievedContextItem {
    #[must_use]
    pub fn snippet(&self) -> &str {
        &self.snippet
    }

    #[must_use]
    pub fn score(&self) -> f32 {
        self.score
    }

    #[must_use]
    pub fn strategies(&self) -> &[RetrievalStrategy] {
        &self.strategies
    }

    #[must_use]
    pub fn provenance(&self) -> &RetrievalProvenance {
        &self.provenance
    }

    #[must_use]
    pub fn was_redacted(&self) -> bool {
        self.redacted
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RetrievalReport {
    items: Vec<RetrievedContextItem>,
    embedding_blocked_reason: Option<String>,
    freshness_blocked_reasons: Vec<String>,
}

impl RetrievalReport {
    #[must_use]
    pub fn items(&self) -> &[RetrievedContextItem] {
        &self.items
    }

    #[must_use]
    pub fn embedding_blocked_reason(&self) -> Option<&str> {
        self.embedding_blocked_reason.as_deref()
    }

    #[must_use]
    pub fn freshness_blocked_reasons(&self) -> &[String] {
        &self.freshness_blocked_reasons
    }
}

pub struct HybridRepoRetriever<'a> {
    index: &'a RepoCodeIndexSnapshot,
    embeddings: Option<&'a dyn LocalEmbeddingBackend>,
}

impl<'a> HybridRepoRetriever<'a> {
    #[must_use]
    pub fn new(index: &'a RepoCodeIndexSnapshot) -> Self {
        Self {
            index,
            embeddings: None,
        }
    }

    #[must_use]
    pub fn with_embeddings(mut self, embeddings: &'a dyn LocalEmbeddingBackend) -> Self {
        self.embeddings = Some(embeddings);
        self
    }

    pub fn retrieve(
        &self,
        query: &str,
        max_items: usize,
        freshness: &RepoIndexFreshnessReport,
    ) -> RetrievalReport {
        if !freshness.is_fresh() {
            return RetrievalReport {
                items: Vec::new(),
                embedding_blocked_reason: None,
                freshness_blocked_reasons: freshness.reasons().to_vec(),
            };
        }
        let terms = query_terms(query);
        let newest = self
            .index
            .documents()
            .iter()
            .filter_map(IndexedCodeDocument::modified_unix_secs)
            .max()
            .unwrap_or_default();
        let (query_embedding, embedding_blocked_reason) = self.query_embedding(query);
        let mut scored = self
            .index
            .documents()
            .iter()
            .filter_map(|document| {
                score_document(
                    document,
                    &terms,
                    newest,
                    self.index.git().dirty_paths(),
                    query_embedding.as_deref(),
                    self.embeddings,
                    self.index.generation_id(),
                )
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.provenance.path.cmp(&right.provenance.path))
        });
        scored.truncate(max_items);
        RetrievalReport {
            items: scored,
            embedding_blocked_reason,
            freshness_blocked_reasons: Vec::new(),
        }
    }

    fn query_embedding(&self, query: &str) -> (Option<Vec<f32>>, Option<String>) {
        let Some(backend) = self.embeddings else {
            return (None, None);
        };
        if backend.locality() != EmbeddingBackendLocality::Local {
            return (None, Some("embedding_backend_not_local".into()));
        }
        match backend.embed(query) {
            Ok(vector) if !vector.is_empty() => (Some(vector), None),
            Ok(_) => (None, Some("embedding_vector_empty".into())),
            Err(reason) => (None, Some(reason)),
        }
    }
}

fn score_document(
    document: &IndexedCodeDocument,
    terms: &[String],
    newest: u64,
    dirty_paths: &[String],
    query_embedding: Option<&[f32]>,
    embeddings: Option<&dyn LocalEmbeddingBackend>,
    generation: &str,
) -> Option<RetrievedContextItem> {
    let path_lower = document.path().to_ascii_lowercase();
    let content_lower = document.content().to_ascii_lowercase();
    let mut score = 0.0;
    let mut strategies = Vec::new();
    let lexical_hits = terms
        .iter()
        .filter(|term| path_lower.contains(term.as_str()) || content_lower.contains(term.as_str()))
        .count();
    if lexical_hits > 0 {
        score += lexical_hits as f32 * 20.0;
        strategies.push(RetrievalStrategy::Lexical);
    }
    let symbol_hit = document.symbols().iter().any(|symbol| {
        terms
            .iter()
            .any(|term| symbol.name().to_ascii_lowercase().contains(term))
    });
    if symbol_hit {
        score += 80.0;
        strategies.push(RetrievalStrategy::Symbol);
    }
    let dependency_hit = document.dependency_hints().iter().any(|hint| {
        let hint = hint.to_ascii_lowercase();
        terms.iter().any(|term| hint.contains(term))
    });
    if dependency_hit {
        score += 40.0;
        strategies.push(RetrievalStrategy::Dependency);
    }
    if dirty_paths.iter().any(|path| path == document.path())
        || document
            .modified_unix_secs()
            .is_some_and(|modified| modified == newest)
    {
        score += 10.0;
        strategies.push(RetrievalStrategy::Recency);
    }
    if let (Some(query_vector), Some(backend)) = (query_embedding, embeddings)
        && let Ok(document_vector) = backend.embed(document.content())
        && let Some(similarity) = cosine_similarity(query_vector, &document_vector)
    {
        score += similarity.max(0.0) * 50.0;
        strategies.push(RetrievalStrategy::LocalEmbedding);
    }
    if score <= 0.0 {
        return None;
    }
    let (snippet, start_line, end_line) = snippet(document.content(), terms);
    let redaction = redact_repository_context(&snippet);
    Some(RetrievedContextItem {
        snippet: redaction.value().to_string(),
        score,
        strategies,
        redacted: redaction.redacted(),
        provenance: RetrievalProvenance {
            path: document.path().to_string(),
            content_hash: document.content_hash().to_string(),
            index_generation: generation.to_string(),
            start_line,
            end_line,
        },
    })
}

fn snippet(content: &str, terms: &[String]) -> (String, usize, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let match_index = lines
        .iter()
        .position(|line| {
            let lower = line.to_ascii_lowercase();
            terms.iter().any(|term| lower.contains(term))
        })
        .unwrap_or(0);
    let start = match_index.saturating_sub(2);
    let end = (match_index + 3).min(lines.len());
    (lines[start..end].join("\n"), start + 1, end.max(start + 1))
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() > 2)
        .collect()
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> Option<f32> {
    if left.len() != right.len() || left.is_empty() {
        return None;
    }
    let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    (left_norm > 0.0 && right_norm > 0.0).then(|| dot / (left_norm * right_norm))
}
