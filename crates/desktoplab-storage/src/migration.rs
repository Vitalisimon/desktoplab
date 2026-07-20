#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReport {
    schema_version: u32,
    applied_migrations: u32,
    migrations: Vec<MigrationStatus>,
}

impl MigrationReport {
    #[must_use]
    pub fn new(schema_version: u32, applied_migrations: u32) -> Self {
        Self {
            schema_version,
            applied_migrations,
            migrations: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_migrations(mut self, migrations: Vec<MigrationStatus>) -> Self {
        self.migrations = migrations;
        self
    }

    #[must_use]
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub fn applied_migrations(&self) -> u32 {
        self.applied_migrations
    }

    #[must_use]
    pub fn migrations(&self) -> &[MigrationStatus] {
        &self.migrations
    }
}

pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationDescriptor {
    id: &'static str,
    version: u32,
    checksum: &'static str,
    description: &'static str,
    reversibility_class: &'static str,
}

impl MigrationDescriptor {
    #[must_use]
    pub const fn new(
        id: &'static str,
        version: u32,
        checksum: &'static str,
        description: &'static str,
        reversibility_class: &'static str,
    ) -> Self {
        Self {
            id,
            version,
            checksum,
            description,
            reversibility_class,
        }
    }

    #[must_use]
    pub fn id(&self) -> &'static str {
        self.id
    }

    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }

    #[must_use]
    pub fn checksum(&self) -> &'static str {
        self.checksum
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        self.description
    }

    #[must_use]
    pub fn reversibility_class(&self) -> &'static str {
        self.reversibility_class
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationStatus {
    descriptor: MigrationDescriptor,
    operator_status: &'static str,
}

impl MigrationStatus {
    #[must_use]
    pub fn new(descriptor: MigrationDescriptor, operator_status: &'static str) -> Self {
        Self {
            descriptor,
            operator_status,
        }
    }

    #[must_use]
    pub fn descriptor(&self) -> &MigrationDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn operator_status(&self) -> &'static str {
        self.operator_status
    }
}

pub const MIGRATION_PLAN: [MigrationDescriptor; 4] = [
    MigrationDescriptor::new(
        "migration-001-local-storage-event-log",
        1,
        "migration-001-local-storage-event-log",
        "Create schema migration ledger and event log.",
        "forward_only",
    ),
    MigrationDescriptor::new(
        "migration-002-persistent-settings-store",
        2,
        "migration-002-persistent-settings-store",
        "Create persistent settings store.",
        "forward_only",
    ),
    MigrationDescriptor::new(
        "migration-003-productization-state",
        3,
        "migration-003-productization-state",
        "Create productization state table.",
        "forward_only",
    ),
    MigrationDescriptor::new(
        "migration-004-local-support-archive",
        4,
        "migration-004-local-support-archive-fts5",
        "Create local support records, sync cursors and full-text index.",
        "forward_only",
    ),
];

#[must_use]
pub fn migration_plan() -> &'static [MigrationDescriptor] {
    &MIGRATION_PLAN
}

pub(crate) const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    checksum TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS event_log (
    event_id TEXT PRIMARY KEY,
    stream_id TEXT NOT NULL,
    stream_kind TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    schema_version INTEGER NOT NULL,
    occurred_at TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    actor TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    redaction_status TEXT NOT NULL,
    trust_context TEXT NOT NULL,
    UNIQUE(stream_id, sequence)
);
"#;

pub(crate) const MIGRATION_002: &str = r#"
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_kind TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

pub(crate) const MIGRATION_003: &str = r#"
CREATE TABLE IF NOT EXISTS productization_state (
    kind TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    payload TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(kind, subject_id)
);
"#;

pub(crate) const MIGRATION_004: &str = r#"
CREATE TABLE IF NOT EXISTS support_records (
    repository TEXT NOT NULL,
    kind TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    parent_remote_id TEXT,
    number INTEGER,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    state TEXT NOT NULL,
    author TEXT NOT NULL,
    url TEXT NOT NULL,
    remote_updated_at TEXT NOT NULL,
    provenance TEXT NOT NULL,
    visibility TEXT NOT NULL,
    tombstoned INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(repository, kind, remote_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS support_records_fts USING fts5(
    record_key UNINDEXED,
    title,
    body,
    kind,
    tokenize = 'unicode61'
);

CREATE TABLE IF NOT EXISTS support_sync_state (
    repository TEXT NOT NULL,
    channel TEXT NOT NULL,
    cursor TEXT,
    refresh_state TEXT NOT NULL,
    last_refresh_at INTEGER NOT NULL,
    provenance TEXT NOT NULL,
    PRIMARY KEY(repository, channel)
);

CREATE TABLE IF NOT EXISTS support_tombstones (
    repository TEXT NOT NULL,
    kind TEXT NOT NULL,
    remote_id TEXT NOT NULL,
    provenance TEXT NOT NULL,
    deleted_at INTEGER NOT NULL,
    PRIMARY KEY(repository, kind, remote_id)
);
"#;
