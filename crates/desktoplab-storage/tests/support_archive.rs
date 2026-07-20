use desktoplab_storage::{
    SupportArchive, SupportArchiveError, SupportRecord, SupportRecordKind, SupportSearchQuery,
    SupportSyncPage, SupportTombstone, SupportVisibility,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

const REPO: &str = "desktoplab/desktoplab";

#[test]
fn incremental_sync_resumes_after_reopen_without_duplicates() {
    let fixture = TempDir::new().unwrap();
    let path = fixture.path().join("support.sqlite");
    {
        let archive = SupportArchive::open(&path).unwrap();
        archive
            .apply_page(page(None, Some("cursor-1"), false, all_kinds()))
            .unwrap();
    }
    let archive = SupportArchive::open(&path).unwrap();
    archive
        .apply_page(page(
            Some("cursor-1"),
            Some("cursor-2"),
            true,
            vec![record(
                SupportRecordKind::Issue,
                "issue-1",
                "Crash on startup edited",
                SupportVisibility::Public,
            )],
        ))
        .unwrap();

    let result = archive.search(&query("crash", 120, false)).unwrap();
    assert_eq!(result.records.len(), 1);
    assert_eq!(result.records[0].title(), "Crash on startup edited");
    assert_eq!(
        archive
            .search(&query("indexed", 120, false))
            .unwrap()
            .records
            .len(),
        6
    );
    assert!(result.current);
}

#[test]
fn cursor_mismatch_is_rejected_without_partial_mutation() {
    let archive = SupportArchive::open_in_memory().unwrap();
    archive
        .apply_page(page(
            None,
            Some("cursor-1"),
            false,
            vec![record(
                SupportRecordKind::Issue,
                "issue-1",
                "Original crash",
                SupportVisibility::Public,
            )],
        ))
        .unwrap();
    let error = archive
        .apply_page(page(
            Some("wrong"),
            Some("cursor-2"),
            true,
            vec![record(
                SupportRecordKind::Issue,
                "issue-2",
                "Should not persist",
                SupportVisibility::Public,
            )],
        ))
        .unwrap_err();
    assert!(matches!(error, SupportArchiveError::CursorMismatch { .. }));
    assert!(
        archive
            .search(&query("persist", 100, false))
            .unwrap()
            .records
            .is_empty()
    );
}

#[test]
fn edits_and_tombstones_reconcile_search_and_provenance() {
    let archive = SupportArchive::open_in_memory().unwrap();
    archive
        .apply_page(page(
            None,
            Some("cursor-1"),
            false,
            vec![record(
                SupportRecordKind::Issue,
                "issue-1",
                "Old renderer failure",
                SupportVisibility::Public,
            )],
        ))
        .unwrap();
    let mut update = page(
        Some("cursor-1"),
        Some("cursor-2"),
        true,
        vec![record(
            SupportRecordKind::Issue,
            "issue-1",
            "New compositor failure",
            SupportVisibility::Public,
        )],
    );
    update.tombstones.push(SupportTombstone::new(
        REPO,
        SupportRecordKind::Comment,
        "missing-comment",
        "github:events:42",
        101,
    ));
    archive.apply_page(update).unwrap();

    assert!(
        archive
            .search(&query("renderer", 120, false))
            .unwrap()
            .records
            .is_empty()
    );
    assert_eq!(
        archive
            .search(&query("compositor", 120, false))
            .unwrap()
            .records
            .len(),
        1
    );
    let tombstone = archive
        .tombstone(REPO, SupportRecordKind::Comment, "missing-comment")
        .unwrap()
        .unwrap();
    assert_eq!(tombstone.provenance(), "github:events:42");
    assert_eq!(tombstone.deleted_at(), 101);
}

#[test]
fn offline_queries_expose_stale_or_incomplete_refresh_truth() {
    let archive = SupportArchive::open_in_memory().unwrap();
    archive
        .apply_page(page(
            None,
            Some("cursor-1"),
            true,
            vec![record(
                SupportRecordKind::Issue,
                "issue-1",
                "Offline search",
                SupportVisibility::Public,
            )],
        ))
        .unwrap();
    let result = archive.search(&query("offline", 1_000, false)).unwrap();
    assert!(!result.current);
    let json = archive
        .search_json(&query("offline", 1_000, false))
        .unwrap();
    assert!(json.contains("stale_or_incomplete"));
    assert!(json.contains("\"remoteWriteBack\":false"));
}

#[test]
fn public_evidence_excludes_private_support_content() {
    let archive = SupportArchive::open_in_memory().unwrap();
    archive
        .apply_page(page(
            None,
            Some("cursor-1"),
            true,
            vec![
                record(
                    SupportRecordKind::Issue,
                    "public",
                    "Public title",
                    SupportVisibility::Public,
                ),
                record(
                    SupportRecordKind::Issue,
                    "private",
                    "confidential customer body",
                    SupportVisibility::Private,
                ),
            ],
        ))
        .unwrap();
    let evidence = archive.public_evidence_json(REPO).unwrap();
    assert!(evidence.contains("\"issue\":1"));
    assert!(!evidence.contains("confidential"));
    assert!(evidence.contains("\"privateContentIncluded\":false"));
}

#[test]
fn support_archive_sources_stay_below_guardrails() {
    check_logical_line_limit(
        "crates/desktoplab-storage/src/support_archive.rs",
        include_str!("../src/support_archive.rs"),
        330,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-storage/src/support_archive/types.rs",
        include_str!("../src/support_archive/types.rs"),
        230,
    )
    .unwrap();
}

fn all_kinds() -> Vec<SupportRecord> {
    [
        SupportRecordKind::Issue,
        SupportRecordKind::PullRequest,
        SupportRecordKind::Comment,
        SupportRecordKind::Review,
        SupportRecordKind::Check,
        SupportRecordKind::Workflow,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, kind)| {
        let id = if kind == SupportRecordKind::Issue {
            "issue-1".to_string()
        } else {
            format!("record-{index}")
        };
        record(
            kind,
            &id,
            "Shared support metadata",
            SupportVisibility::Public,
        )
    })
    .collect()
}

fn record(
    kind: SupportRecordKind,
    id: &str,
    title: &str,
    visibility: SupportVisibility,
) -> SupportRecord {
    SupportRecord::new(
        REPO,
        kind,
        id,
        None,
        Some(1),
        title,
        "indexed body",
        "open",
        "user",
        "https://example.invalid/record",
        "2026-07-15T00:00:00Z",
        "github:api:v1",
        visibility,
    )
}

fn page(
    before: Option<&str>,
    after: Option<&str>,
    complete: bool,
    records: Vec<SupportRecord>,
) -> SupportSyncPage {
    SupportSyncPage {
        repository: REPO.to_string(),
        channel: "issues".to_string(),
        cursor_before: before.map(str::to_string),
        cursor_after: after.map(str::to_string),
        complete,
        refreshed_at: 100,
        provenance: "github:api:v1".to_string(),
        records,
        tombstones: Vec::new(),
    }
}

fn query(text: &str, now: i64, include_private: bool) -> SupportSearchQuery {
    SupportSearchQuery {
        repository: REPO.to_string(),
        text: text.to_string(),
        include_private,
        now,
        max_age_seconds: 100,
        limit: 20,
        required_channels: vec!["issues".to_string()],
    }
}
