use crate::event::StoredEventRow;
use crate::migration::{
    CURRENT_SCHEMA_VERSION, MIGRATION_001, MIGRATION_002, MIGRATION_003, MIGRATION_004,
    migration_plan,
};
use crate::productization::{ProductizationRecordKind, ProductizationStateRecord};
use crate::secret::reject_secret_like_payload;
use crate::settings::reject_raw_secret_setting;
use crate::{
    EventEnvelope, EventStore, MigrationReport, MigrationStatus, SettingRecord, SettingValue,
    StorageError,
};
use rusqlite::{Connection, params};
use std::path::Path;

pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { connection })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { connection })
    }

    pub fn apply_migrations(&self) -> Result<MigrationReport, StorageError> {
        self.connection.execute_batch(MIGRATION_001)?;
        let mut applied_migrations = 0;

        applied_migrations +=
            self.apply_migration(1, "migration-001-local-storage-event-log", "")?;
        applied_migrations +=
            self.apply_migration(2, "migration-002-persistent-settings-store", MIGRATION_002)?;
        applied_migrations +=
            self.apply_migration(3, "migration-003-productization-state", MIGRATION_003)?;
        applied_migrations +=
            self.apply_migration(4, "migration-004-local-support-archive-fts5", MIGRATION_004)?;

        Ok(
            MigrationReport::new(CURRENT_SCHEMA_VERSION, applied_migrations).with_migrations(
                migration_plan()
                    .iter()
                    .cloned()
                    .map(|descriptor| MigrationStatus::new(descriptor, "applied"))
                    .collect(),
            ),
        )
    }

    pub fn schema_version(&self) -> Result<u32, StorageError> {
        let version = self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, u32>(0),
        )?;

        Ok(version)
    }

    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }

    fn migration_exists(&self, version: u32) -> Result<bool, StorageError> {
        let count = self.connection.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            [version],
            |row| row.get::<_, u32>(0),
        )?;

        Ok(count > 0)
    }

    fn apply_migration(
        &self,
        version: u32,
        checksum: &str,
        sql: &str,
    ) -> Result<u32, StorageError> {
        if self.migration_exists(version)? {
            return Ok(0);
        }

        self.connection.execute_batch(sql)?;
        self.connection.execute(
            "INSERT INTO schema_migrations (version, checksum, applied_at) VALUES (?1, ?2, ?3)",
            params![version, checksum, "1970-01-01T00:00:00Z"],
        )?;

        Ok(1)
    }

    pub fn put_setting(&self, setting: SettingRecord) -> Result<(), StorageError> {
        reject_raw_secret_setting(&setting)?;

        self.connection.execute(
            "INSERT INTO settings (key, value_kind, value, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(key) DO UPDATE SET
                value_kind = excluded.value_kind,
                value = excluded.value,
                updated_at = excluded.updated_at",
            params![
                setting.key(),
                setting.value().kind(),
                setting.value().raw_value(),
                setting.updated_at(),
            ],
        )?;

        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<SettingRecord>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT key, value_kind, value, updated_at FROM settings WHERE key = ?1")?;
        let mut rows = statement.query([key])?;

        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        let key: String = row.get(0)?;
        let value_kind: String = row.get(1)?;
        let value: String = row.get(2)?;
        let updated_at: String = row.get(3)?;

        Ok(Some(SettingRecord::from_storage(
            key,
            SettingValue::from_storage(&value_kind, value)?,
            updated_at,
        )))
    }

    pub fn put_productization_state(
        &self,
        record: ProductizationStateRecord,
    ) -> Result<(), StorageError> {
        self.put_productization_states(&[record])
    }

    pub fn put_productization_states(
        &self,
        records: &[ProductizationStateRecord],
    ) -> Result<(), StorageError> {
        for record in records {
            reject_secret_like_payload(record.payload(), crate::RedactionStatus::Clean)?;
        }

        let transaction = self.connection.unchecked_transaction()?;
        for record in records {
            transaction.execute(
                "INSERT INTO productization_state (kind, subject_id, payload, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(kind, subject_id) DO UPDATE SET
                payload = excluded.payload,
                updated_at = excluded.updated_at",
                params![
                    record.kind().as_str(),
                    record.subject_id(),
                    record.payload(),
                    record.updated_at(),
                ],
            )?;
        }
        transaction.commit()?;

        Ok(())
    }

    pub fn get_productization_state(
        &self,
        kind: ProductizationRecordKind,
        subject_id: &str,
    ) -> Result<Option<ProductizationStateRecord>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT kind, subject_id, payload, updated_at
            FROM productization_state
            WHERE kind = ?1 AND subject_id = ?2",
        )?;
        let mut rows = statement.query(params![kind.as_str(), subject_id])?;

        let Some(row) = rows.next()? else {
            return Ok(None);
        };

        let kind: String = row.get(0)?;
        let subject_id: String = row.get(1)?;
        let payload: String = row.get(2)?;
        let updated_at: String = row.get(3)?;

        Ok(Some(ProductizationStateRecord::from_storage(
            ProductizationRecordKind::from_storage(&kind),
            subject_id,
            payload,
            updated_at,
        )))
    }
}

impl EventStore for SqliteStore {
    fn append_event(&self, event: EventEnvelope) -> Result<(), StorageError> {
        reject_secret_like_payload(event.payload(), event.redaction_status())?;

        self.connection.execute(
            "INSERT INTO event_log (
                event_id,
                stream_id,
                stream_kind,
                sequence,
                schema_version,
                occurred_at,
                recorded_at,
                actor,
                event_type,
                payload,
                correlation_id,
                redaction_status,
                trust_context
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                event.event_id(),
                event.stream_id(),
                event.stream_kind().as_str(),
                event.sequence() as i64,
                event.schema_version(),
                event.occurred_at(),
                event.recorded_at(),
                event.actor(),
                event.event_type(),
                event.payload(),
                event.correlation_id(),
                event.redaction_status().as_str(),
                event.trust_context(),
            ],
        )?;

        Ok(())
    }

    fn replay_stream(&self, stream_id: &str) -> Result<Vec<EventEnvelope>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT
                event_id,
                stream_id,
                stream_kind,
                sequence,
                schema_version,
                occurred_at,
                recorded_at,
                actor,
                event_type,
                payload,
                correlation_id,
                redaction_status,
                trust_context
            FROM event_log
            WHERE stream_id = ?1
            ORDER BY sequence ASC",
        )?;

        let rows = statement.query_map([stream_id], |row| {
            Ok(StoredEventRow {
                event_id: row.get(0)?,
                stream_id: row.get(1)?,
                stream_kind: row.get(2)?,
                sequence: row.get::<_, i64>(3)? as u64,
                schema_version: row.get(4)?,
                occurred_at: row.get(5)?,
                recorded_at: row.get(6)?,
                actor: row.get(7)?,
                event_type: row.get(8)?,
                payload: row.get(9)?,
                correlation_id: row.get(10)?,
                redaction_status: row.get(11)?,
                trust_context: row.get(12)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(EventEnvelope::from_storage(row?));
        }

        Ok(events)
    }
}
