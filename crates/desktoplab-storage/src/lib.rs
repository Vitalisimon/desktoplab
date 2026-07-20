#![forbid(unsafe_code)]

mod archive;
mod atomic_file;
mod error;
mod event;
mod migration;
mod productization;
mod secret;
mod settings;
mod sqlite;
mod support_archive;
#[cfg(windows)]
mod windows_acl;

pub use archive::{
    ArchiveFormat, ArchiveImportError, ArchiveLimits, ArchiveReport, import_archive,
};
pub use atomic_file::{AtomicFileStore, CrossProcessLock, LockError, PrivateFileMode};
pub use error::{SecretRejected, StorageError};
pub use event::{EventEnvelope, EventStore, RedactionStatus, StreamKind};
pub use migration::{MigrationDescriptor, MigrationReport, MigrationStatus, migration_plan};
pub use productization::{ProductizationRecordKind, ProductizationStateRecord};
pub use settings::{SettingRecord, SettingValue};
pub use sqlite::SqliteStore;
pub use support_archive::{
    SupportArchive, SupportArchiveError, SupportRecord, SupportRecordKind, SupportSearchQuery,
    SupportSearchResult, SupportSyncPage, SupportSyncState, SupportTombstone, SupportVisibility,
};
