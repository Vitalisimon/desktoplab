#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportRecordKind {
    Issue,
    PullRequest,
    Comment,
    Review,
    Check,
    Workflow,
}

impl SupportRecordKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Issue => "issue",
            Self::PullRequest => "pull_request",
            Self::Comment => "comment",
            Self::Review => "review",
            Self::Check => "check",
            Self::Workflow => "workflow",
        }
    }

    pub(super) fn from_storage(value: &str) -> Option<Self> {
        match value {
            "issue" => Some(Self::Issue),
            "pull_request" => Some(Self::PullRequest),
            "comment" => Some(Self::Comment),
            "review" => Some(Self::Review),
            "check" => Some(Self::Check),
            "workflow" => Some(Self::Workflow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportVisibility {
    Public,
    Private,
}

impl SupportVisibility {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }

    pub(super) fn from_storage(value: &str) -> Self {
        if value == "private" {
            Self::Private
        } else {
            Self::Public
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportRecord {
    pub(super) repository: String,
    pub(super) kind: SupportRecordKind,
    pub(super) remote_id: String,
    pub(super) parent_remote_id: Option<String>,
    pub(super) number: Option<i64>,
    pub(super) title: String,
    pub(super) body: String,
    pub(super) state: String,
    pub(super) author: String,
    pub(super) url: String,
    pub(super) remote_updated_at: String,
    pub(super) provenance: String,
    pub(super) visibility: SupportVisibility,
}

impl SupportRecord {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        repository: impl Into<String>,
        kind: SupportRecordKind,
        remote_id: impl Into<String>,
        parent_remote_id: Option<String>,
        number: Option<i64>,
        title: impl Into<String>,
        body: impl Into<String>,
        state: impl Into<String>,
        author: impl Into<String>,
        url: impl Into<String>,
        remote_updated_at: impl Into<String>,
        provenance: impl Into<String>,
        visibility: SupportVisibility,
    ) -> Self {
        Self {
            repository: repository.into(),
            kind,
            remote_id: remote_id.into(),
            parent_remote_id,
            number,
            title: title.into(),
            body: body.into(),
            state: state.into(),
            author: author.into(),
            url: url.into(),
            remote_updated_at: remote_updated_at.into(),
            provenance: provenance.into(),
            visibility,
        }
    }

    #[must_use]
    pub fn kind(&self) -> SupportRecordKind {
        self.kind
    }
    #[must_use]
    pub fn remote_id(&self) -> &str {
        &self.remote_id
    }
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
    #[must_use]
    pub fn visibility(&self) -> SupportVisibility {
        self.visibility
    }

    pub(super) fn key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.repository,
            self.kind.as_str(),
            self.remote_id
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportTombstone {
    pub(super) repository: String,
    pub(super) kind: SupportRecordKind,
    pub(super) remote_id: String,
    pub(super) provenance: String,
    pub(super) deleted_at: i64,
}

impl SupportTombstone {
    #[must_use]
    pub fn new(
        repository: impl Into<String>,
        kind: SupportRecordKind,
        remote_id: impl Into<String>,
        provenance: impl Into<String>,
        deleted_at: i64,
    ) -> Self {
        Self {
            repository: repository.into(),
            kind,
            remote_id: remote_id.into(),
            provenance: provenance.into(),
            deleted_at,
        }
    }

    #[must_use]
    pub fn provenance(&self) -> &str {
        &self.provenance
    }

    #[must_use]
    pub fn deleted_at(&self) -> i64 {
        self.deleted_at
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportSyncPage {
    pub repository: String,
    pub channel: String,
    pub cursor_before: Option<String>,
    pub cursor_after: Option<String>,
    pub complete: bool,
    pub refreshed_at: i64,
    pub provenance: String,
    pub records: Vec<SupportRecord>,
    pub tombstones: Vec<SupportTombstone>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportSearchQuery {
    pub repository: String,
    pub text: String,
    pub include_private: bool,
    pub now: i64,
    pub max_age_seconds: i64,
    pub limit: usize,
    pub required_channels: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportSyncState {
    pub channel: String,
    pub cursor: Option<String>,
    pub refresh_state: String,
    pub last_refresh_at: i64,
    pub provenance: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportSearchResult {
    pub records: Vec<SupportRecord>,
    pub sync: Vec<SupportSyncState>,
    pub current: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupportArchiveError {
    Storage(String),
    InvalidRecord(String),
    CursorMismatch {
        expected: Option<String>,
        received: Option<String>,
    },
}

impl From<crate::StorageError> for SupportArchiveError {
    fn from(error: crate::StorageError) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<rusqlite::Error> for SupportArchiveError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.to_string())
    }
}
