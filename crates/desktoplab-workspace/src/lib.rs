#![forbid(unsafe_code)]

mod api;
mod checkpoint;
mod file_preview;
mod file_tree;
mod git;
mod index;
mod indexing;
mod inspection;
mod intelligence;
mod memory;
mod policy;
mod product_git;
mod refresh;
mod registry;
mod retrieval;
mod retrieval_freshness;
mod search;
mod search_pattern;
mod syntax_index;
mod test_detection;
mod worktree;

pub use api::{
    WorkspaceApiError, WorkspaceApiErrorCode, WorkspaceApiService, WorkspaceApiSnapshot,
    WorkspaceApiState,
};
pub use checkpoint::{CheckpointPlan, CheckpointStatus};
pub use file_preview::{FilePreview, FilePreviewLimits, FilePreviewState};
pub use file_tree::{
    FileTreeEntry, FileTreeEntryKind, FileTreeProtection, WorkspaceFileTree,
    WorkspaceFileTreeLimits,
};
pub use git::{
    GitDiff, GitRepository, GitStatus, GitStatusFile, RepositoryIdentity, WorkspaceGitError,
};
pub use index::{
    WorkspaceIndex, WorkspaceIndexEntry, WorkspaceIndexLimits, WorkspaceIndexSnapshot,
};
pub use indexing::{
    IndexedCodeDocument, RepoCodeIndexSnapshot, RepoCodeIndexer, RepoGitMetadata, RepoIndexLimits,
};
pub use inspection::{RepositoryInspection, RepositoryInspector};
pub use intelligence::{WorkspaceIntelligenceApi, WorkspaceIntelligenceSnapshot};
pub use memory::{MemoryId, MemoryRecord, MemoryVisibility, WorkspaceMemoryStore};
pub use policy::{
    ClassifiedWorkspacePaths, PathClassification, PolicyOverrideRecord, WorkspacePolicyClassifier,
};
pub use product_git::{
    CommitApproval, CommitOperation, GitProductizationOutcome, ParallelAgentRoute,
    ParallelAgentRouter, ProductWorktreeManager, PushApproval, PushOperation, RollbackApproval,
    RollbackOperation, RollbackPreview, SavePoint, SavePointManager, SessionIntent,
};
pub use refresh::{ContextRefreshReport, ContextRefreshScheduler};
pub use registry::{WorkspaceRegistration, WorkspaceRegistry};
pub use retrieval::{
    EmbeddingBackendLocality, HybridRepoRetriever, LocalEmbeddingBackend, RetrievalProvenance,
    RetrievalReport, RetrievalStrategy, RetrievedContextItem,
};
pub use retrieval_freshness::{
    RepoIndexFreshnessGuard, RepoIndexFreshnessReport, RepoIndexFreshnessState,
};
pub use search::{
    WorkspaceFileEntry, WorkspaceFileSafety, WorkspaceSearch, WorkspaceSearchHit,
    WorkspaceSearchLimits, WorkspaceSearchReport,
};
pub use syntax_index::{CodeReference, CodeSymbol, SyntaxIndex};
pub use test_detection::{
    DetectedTestCommand, TestCommandConfidence, TestCommandDetector, TestCommandSet,
};
pub use worktree::{IsolationDecision, ParallelExecutionKind, WorktreePolicy};
