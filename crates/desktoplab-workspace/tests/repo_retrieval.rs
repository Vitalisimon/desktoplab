use desktoplab_workspace::{
    EmbeddingBackendLocality, HybridRepoRetriever, LocalEmbeddingBackend, RepoCodeIndexer,
    RepoIndexFreshnessGuard, RepoIndexLimits, RetrievalStrategy,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use xtask::check_logical_line_limit;

#[test]
fn index_uses_gitignore_ast_symbols_dependencies_and_git_metadata() {
    let repo = repository();
    write(repo.path(), ".gitignore", "ignored/\n");
    write(
        repo.path(),
        "src/lib.rs",
        "use crate::payments::Gateway;\npub struct CheckoutService;\nimpl CheckoutService { pub fn charge(&self) {} }\n",
    );
    write(
        repo.path(),
        "web/client.ts",
        "import { api } from './api';\nexport class BillingClient { charge() { return api(); } }\n",
    );
    write(repo.path(), "ignored/secret.rs", "struct MustNotAppear;\n");
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "seed"]);
    write(repo.path(), "src/lib.rs", "pub struct CheckoutService;\n");

    let index = RepoCodeIndexer::new(RepoIndexLimits::new(100, 100_000))
        .build(repo.path())
        .unwrap();

    assert!(
        index
            .documents()
            .iter()
            .all(|document| document.path() != "ignored/secret.rs")
    );
    let rust = index
        .documents()
        .iter()
        .find(|document| document.path() == "src/lib.rs")
        .unwrap();
    assert!(
        rust.symbols()
            .iter()
            .any(|symbol| symbol.name() == "CheckoutService")
    );
    assert_eq!(index.git().branch(), Some("main"));
    assert!(index.git().head().is_some());
    assert!(
        index
            .git()
            .dirty_paths()
            .contains(&"src/lib.rs".to_string())
    );
    assert_eq!(index.generation_id().len(), 64);
}

#[test]
fn hybrid_retrieval_returns_ranked_snippets_with_provenance() {
    let repo = repository();
    write(
        repo.path(),
        "src/checkout.rs",
        "pub struct CheckoutService;\nimpl CheckoutService { pub fn charge(&self) {} }\n",
    );
    write(repo.path(), "src/other.rs", "pub fn unrelated() {}\n");
    let index = RepoCodeIndexer::new(RepoIndexLimits::new(100, 100_000))
        .build(repo.path())
        .unwrap();
    let embeddings = FixtureEmbeddings {
        locality: EmbeddingBackendLocality::Local,
    };
    let freshness = RepoIndexFreshnessGuard::validate(&index, repo.path());

    let report = HybridRepoRetriever::new(&index)
        .with_embeddings(&embeddings)
        .retrieve("CheckoutService charge", 3, &freshness);
    let first = &report.items()[0];

    assert_eq!(first.provenance().path(), "src/checkout.rs");
    assert_eq!(first.provenance().content_hash().len(), 64);
    assert_eq!(first.provenance().index_generation(), index.generation_id());
    assert!(first.provenance().start_line() >= 1);
    assert!(first.snippet().contains("CheckoutService"));
    assert!(first.strategies().contains(&RetrievalStrategy::Lexical));
    assert!(first.strategies().contains(&RetrievalStrategy::Symbol));
    assert!(
        first
            .strategies()
            .contains(&RetrievalStrategy::LocalEmbedding)
    );
}

#[test]
fn external_embedding_backend_is_blocked_without_invocation() {
    let repo = repository();
    write(repo.path(), "src/lib.rs", "pub fn local_only() {}\n");
    let index = RepoCodeIndexer::new(RepoIndexLimits::new(100, 100_000))
        .build(repo.path())
        .unwrap();
    let embeddings = FixtureEmbeddings {
        locality: EmbeddingBackendLocality::External,
    };
    let freshness = RepoIndexFreshnessGuard::validate(&index, repo.path());

    let report = HybridRepoRetriever::new(&index)
        .with_embeddings(&embeddings)
        .retrieve("local_only", 2, &freshness);

    assert_eq!(
        report.embedding_blocked_reason(),
        Some("embedding_backend_not_local")
    );
    assert!(report.items().iter().all(|item| {
        !item
            .strategies()
            .contains(&RetrievalStrategy::LocalEmbedding)
    }));
}

#[test]
fn repo_retrieval_sources_stay_below_line_guards() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-workspace/src/indexing.rs",
            include_str!("../src/indexing.rs"),
            340,
        ),
        (
            "crates/desktoplab-workspace/src/retrieval.rs",
            include_str!("../src/retrieval.rs"),
            340,
        ),
        (
            "crates/desktoplab-workspace/src/syntax_index.rs",
            include_str!("../src/syntax_index.rs"),
            240,
        ),
    ] {
        check_logical_line_limit(path, source, limit)
            .expect("repo retrieval source should stay focused");
    }
}

struct FixtureEmbeddings {
    locality: EmbeddingBackendLocality,
}

impl LocalEmbeddingBackend for FixtureEmbeddings {
    fn locality(&self) -> EmbeddingBackendLocality {
        self.locality
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let text = text.to_ascii_lowercase();
        Ok(vec![
            if text.contains("checkout") { 1.0 } else { 0.0 },
            if text.contains("unrelated") { 1.0 } else { 0.0 },
        ])
    }
}

fn repository() -> tempfile::TempDir {
    let repo = tempdir().unwrap();
    run_git(repo.path(), &["init", "-b", "main"]);
    run_git(
        repo.path(),
        &["config", "user.email", "desktoplab@example.invalid"],
    );
    run_git(repo.path(), &["config", "user.name", "DesktopLab Test"]);
    repo
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn run_git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}
