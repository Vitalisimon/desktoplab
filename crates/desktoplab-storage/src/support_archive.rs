mod types;
pub use types::*;

use std::path::Path;

use rusqlite::{OptionalExtension, params};
use serde_json::json;

use crate::SqliteStore;

pub struct SupportArchive {
    store: SqliteStore,
}

impl SupportArchive {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SupportArchiveError> {
        let store = SqliteStore::open(path)?;
        store.apply_migrations()?;
        Ok(Self { store })
    }

    pub fn open_in_memory() -> Result<Self, SupportArchiveError> {
        let store = SqliteStore::open_in_memory()?;
        store.apply_migrations()?;
        Ok(Self { store })
    }

    pub fn apply_page(&self, page: SupportSyncPage) -> Result<(), SupportArchiveError> {
        validate_page(&page)?;
        let current = self.cursor(&page.repository, &page.channel)?;
        if current != page.cursor_before {
            return Err(SupportArchiveError::CursorMismatch {
                expected: current,
                received: page.cursor_before,
            });
        }
        let transaction = self.store.connection().unchecked_transaction()?;
        for record in &page.records {
            upsert_record(&transaction, record)?;
        }
        for tombstone in &page.tombstones {
            apply_tombstone(&transaction, tombstone)?;
        }
        transaction.execute(
            "INSERT INTO support_sync_state (repository, channel, cursor, refresh_state, last_refresh_at, provenance)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(repository, channel) DO UPDATE SET cursor=excluded.cursor, refresh_state=excluded.refresh_state,
             last_refresh_at=excluded.last_refresh_at, provenance=excluded.provenance",
            params![page.repository, page.channel, page.cursor_after, if page.complete { "complete" } else { "in_progress" }, page.refreshed_at, page.provenance],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        query: &SupportSearchQuery,
    ) -> Result<SupportSearchResult, SupportArchiveError> {
        if query.repository.trim().is_empty()
            || query.text.trim().is_empty()
            || query.max_age_seconds == 0
        {
            return Err(SupportArchiveError::InvalidRecord(
                "invalid_search_query".to_string(),
            ));
        }
        let fts_query = fts_query(&query.text);
        if fts_query.is_empty() {
            return Err(SupportArchiveError::InvalidRecord(
                "empty_search_terms".to_string(),
            ));
        }
        let mut statement = self.store.connection().prepare(
            "SELECT r.repository,r.kind,r.remote_id,r.parent_remote_id,r.number,r.title,r.body,r.state,
                    r.author,r.url,r.remote_updated_at,r.provenance,r.visibility
             FROM support_records_fts f JOIN support_records r
               ON f.record_key=(r.repository || ':' || r.kind || ':' || r.remote_id)
             WHERE support_records_fts MATCH ?1 AND r.repository=?2 AND r.tombstoned=0
               AND (?3=1 OR r.visibility='public')
             ORDER BY bm25(support_records_fts) LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![
                fts_query,
                query.repository,
                query.include_private,
                query.limit.clamp(1, 100) as i64
            ],
            row_to_record,
        )?;
        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        let sync = self.sync_states(&query.repository)?;
        let current = !query.required_channels.is_empty()
            && query.required_channels.iter().all(|channel| {
                sync.iter().any(|state| {
                    state.channel == *channel
                        && state.refresh_state == "complete"
                        && query.now >= state.last_refresh_at
                        && query.now - state.last_refresh_at <= query.max_age_seconds
                })
            });
        Ok(SupportSearchResult {
            records,
            sync,
            current,
        })
    }

    pub fn search_json(&self, query: &SupportSearchQuery) -> Result<String, SupportArchiveError> {
        let result = self.search(query)?;
        let records = result.records.iter().map(|record| json!({
            "kind":record.kind.as_str(), "remoteId":record.remote_id, "parentRemoteId":record.parent_remote_id,
            "number":record.number, "title":record.title, "body":record.body, "state":record.state,
            "author":record.author, "url":record.url, "remoteUpdatedAt":record.remote_updated_at,
            "provenance":record.provenance, "visibility":record.visibility.as_str(),
        })).collect::<Vec<_>>();
        let sync = result.sync.iter().map(|state| json!({
            "channel":state.channel, "cursor":state.cursor, "refreshState":state.refresh_state,
            "lastRefreshAt":state.last_refresh_at, "provenance":state.provenance,
        })).collect::<Vec<_>>();
        Ok(json!({
            "kind":"desktoplab.local-support-search", "schemaVersion":1,
            "repository":query.repository, "current":result.current,
            "freshness":if result.current { "current" } else { "stale_or_incomplete" },
            "sync":sync, "records":records, "remoteWriteBack":false,
        })
        .to_string())
    }

    pub fn public_evidence_json(&self, repository: &str) -> Result<String, SupportArchiveError> {
        let mut statement = self.store.connection().prepare(
            "SELECT kind, COUNT(*) FROM support_records WHERE repository=?1 AND tombstoned=0 AND visibility='public' GROUP BY kind ORDER BY kind",
        )?;
        let rows = statement.query_map([repository], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut counts = serde_json::Map::new();
        for row in rows {
            let (kind, count) = row?;
            counts.insert(kind, json!(count));
        }
        Ok(json!({
            "kind":"desktoplab.public-support-archive-evidence", "schemaVersion":1,
            "repository":repository, "counts":counts, "contentIncluded":false,
            "privateContentIncluded":false, "remoteWriteBack":false,
        })
        .to_string())
    }

    pub fn tombstone(
        &self,
        repository: &str,
        kind: SupportRecordKind,
        remote_id: &str,
    ) -> Result<Option<SupportTombstone>, SupportArchiveError> {
        Ok(self
            .store
            .connection()
            .query_row(
                "SELECT repository,kind,remote_id,provenance,deleted_at FROM support_tombstones
             WHERE repository=?1 AND kind=?2 AND remote_id=?3",
                params![repository, kind.as_str(), remote_id],
                |row| {
                    Ok(SupportTombstone {
                        repository: row.get(0)?,
                        kind,
                        remote_id: row.get(2)?,
                        provenance: row.get(3)?,
                        deleted_at: row.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    fn cursor(
        &self,
        repository: &str,
        channel: &str,
    ) -> Result<Option<String>, SupportArchiveError> {
        let cursor: Option<Option<String>> = self
            .store
            .connection()
            .query_row(
                "SELECT cursor FROM support_sync_state WHERE repository=?1 AND channel=?2",
                params![repository, channel],
                |row| row.get(0),
            )
            .optional()?;
        Ok(cursor.flatten())
    }

    fn sync_states(&self, repository: &str) -> Result<Vec<SupportSyncState>, SupportArchiveError> {
        let mut statement = self.store.connection().prepare(
            "SELECT channel,cursor,refresh_state,last_refresh_at,provenance FROM support_sync_state WHERE repository=?1 ORDER BY channel",
        )?;
        let rows = statement.query_map([repository], |row| {
            Ok(SupportSyncState {
                channel: row.get(0)?,
                cursor: row.get(1)?,
                refresh_state: row.get(2)?,
                last_refresh_at: row.get(3)?,
                provenance: row.get(4)?,
            })
        })?;
        let mut states = Vec::new();
        for row in rows {
            states.push(row?);
        }
        Ok(states)
    }
}

fn validate_page(page: &SupportSyncPage) -> Result<(), SupportArchiveError> {
    if page.repository.trim().is_empty()
        || page.repository.contains(':')
        || page.channel.trim().is_empty()
        || page.provenance.trim().is_empty()
    {
        return Err(SupportArchiveError::InvalidRecord(
            "sync_page_identity_missing".to_string(),
        ));
    }
    if page.records.iter().any(|record| {
        record.repository != page.repository
            || record.remote_id.trim().is_empty()
            || record.remote_id.contains(':')
            || record.provenance.trim().is_empty()
    }) || page.tombstones.iter().any(|item| {
        item.repository != page.repository
            || item.remote_id.trim().is_empty()
            || item.remote_id.contains(':')
            || item.provenance.trim().is_empty()
    }) {
        return Err(SupportArchiveError::InvalidRecord(
            "record_identity_invalid".to_string(),
        ));
    }
    Ok(())
}

fn upsert_record(
    connection: &rusqlite::Connection,
    record: &SupportRecord,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "INSERT INTO support_records (repository,kind,remote_id,parent_remote_id,number,title,body,state,author,url,remote_updated_at,provenance,visibility,tombstoned)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,0)
         ON CONFLICT(repository,kind,remote_id) DO UPDATE SET parent_remote_id=excluded.parent_remote_id,number=excluded.number,
         title=excluded.title,body=excluded.body,state=excluded.state,author=excluded.author,url=excluded.url,
         remote_updated_at=excluded.remote_updated_at,provenance=excluded.provenance,visibility=excluded.visibility,tombstoned=0",
        params![record.repository,record.kind.as_str(),record.remote_id,record.parent_remote_id,record.number,record.title,record.body,
            record.state,record.author,record.url,record.remote_updated_at,record.provenance,record.visibility.as_str()],
    )?;
    connection.execute(
        "DELETE FROM support_tombstones WHERE repository=?1 AND kind=?2 AND remote_id=?3",
        params![record.repository, record.kind.as_str(), record.remote_id],
    )?;
    connection.execute(
        "DELETE FROM support_records_fts WHERE record_key=?1",
        [record.key()],
    )?;
    connection.execute(
        "INSERT INTO support_records_fts (record_key,title,body,kind) VALUES (?1,?2,?3,?4)",
        params![
            record.key(),
            record.title,
            record.body,
            record.kind.as_str()
        ],
    )?;
    Ok(())
}

fn apply_tombstone(
    connection: &rusqlite::Connection,
    tombstone: &SupportTombstone,
) -> Result<(), rusqlite::Error> {
    connection.execute(
        "INSERT INTO support_tombstones (repository,kind,remote_id,provenance,deleted_at) VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(repository,kind,remote_id) DO UPDATE SET provenance=excluded.provenance,deleted_at=excluded.deleted_at",
        params![tombstone.repository,tombstone.kind.as_str(),tombstone.remote_id,tombstone.provenance,tombstone.deleted_at],
    )?;
    connection.execute(
        "UPDATE support_records SET tombstoned=1,provenance=?4 WHERE repository=?1 AND kind=?2 AND remote_id=?3",
        params![tombstone.repository,tombstone.kind.as_str(),tombstone.remote_id,tombstone.provenance],
    )?;
    connection.execute(
        "DELETE FROM support_records_fts WHERE record_key=?1",
        [format!(
            "{}:{}:{}",
            tombstone.repository,
            tombstone.kind.as_str(),
            tombstone.remote_id
        )],
    )?;
    Ok(())
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SupportRecord> {
    let kind: String = row.get(1)?;
    let kind = SupportRecordKind::from_storage(&kind).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unknown support record kind",
            )
            .into(),
        )
    })?;
    Ok(SupportRecord {
        repository: row.get(0)?,
        kind,
        remote_id: row.get(2)?,
        parent_remote_id: row.get(3)?,
        number: row.get(4)?,
        title: row.get(5)?,
        body: row.get(6)?,
        state: row.get(7)?,
        author: row.get(8)?,
        url: row.get(9)?,
        remote_updated_at: row.get(10)?,
        provenance: row.get(11)?,
        visibility: SupportVisibility::from_storage(&row.get::<_, String>(12)?),
    })
}

fn fts_query(value: &str) -> String {
    value
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|term| !term.is_empty())
        .take(12)
        .map(|term| format!("\"{}\"*", term.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" AND ")
}
