use std::fs;

use desktoplab_workspace::{WorkspaceFileSafety, WorkspaceSearch, WorkspaceSearchLimits};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn workspace_search_excludes_git_generated_binary_and_secret_content() {
    let repo = search_fixture();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(64, 16, 4096));

    let report = search.search(repo.path(), "composer").unwrap();
    let paths = report
        .matches()
        .iter()
        .map(|hit| hit.path())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"apps/desktop/src/features/productization/AgentComposer.tsx"));
    let composer = report
        .matches()
        .iter()
        .find(|hit| hit.path().ends_with("AgentComposer.tsx"))
        .unwrap();
    assert_eq!(composer.line_number(), Some(1));
    assert!(!paths.iter().any(|path| path.contains(".git")));
    assert!(!paths.iter().any(|path| path.contains("node_modules")));
    assert!(
        report
            .matches()
            .iter()
            .all(|hit| !hit.preview().contains("sk-secret"))
    );
}

#[test]
fn workspace_file_listing_includes_language_size_and_safety() {
    let repo = search_fixture();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(64, 16, 4096));

    let listing = search.list_files(repo.path()).unwrap();
    let composer = listing
        .iter()
        .find(|entry| entry.path() == "apps/desktop/src/features/productization/AgentComposer.tsx")
        .unwrap();
    let env = listing.iter().find(|entry| entry.path() == ".env").unwrap();

    assert_eq!(composer.language(), Some("typescript"));
    assert!(composer.size_bytes() > 0);
    assert_eq!(composer.safety(), WorkspaceFileSafety::Readable);
    assert_eq!(env.safety(), WorkspaceFileSafety::Protected);
}

#[test]
fn ranked_context_prefers_prompt_terms_and_entrypoints() {
    let repo = search_fixture();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(64, 16, 4096));

    let composer = search
        .ranked_context_paths(repo.path(), "trova composer", &[], 3)
        .unwrap();
    let overview = search
        .ranked_context_paths(repo.path(), "spiega questa repo", &["src/lib.rs"], 4)
        .unwrap();

    assert_eq!(
        composer[0],
        "apps/desktop/src/features/productization/AgentComposer.tsx"
    );
    assert!(overview.contains(&"README.md".to_string()));
    assert!(overview.contains(&"package.json".to_string()));
    assert!(overview.contains(&"src/lib.rs".to_string()));
}

#[test]
fn workspace_search_reports_truncation_on_large_repositories() {
    let repo = TempDir::new().unwrap();
    for index in 0..12 {
        fs::write(repo.path().join(format!("file-{index}.txt")), "needle\n").unwrap();
    }
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(4, 3, 1024));

    let report = search.search(repo.path(), "needle").unwrap();

    assert!(report.truncated());
    assert_eq!(report.matches().len(), 3);
}

#[test]
fn workspace_search_returns_each_matching_line_in_the_same_file() {
    let repo = TempDir::new().unwrap();
    fs::write(
        repo.path().join("repeated.rs"),
        "fn first() { target(); }\nfn middle() {}\nfn last() { target(); }\n",
    )
    .unwrap();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(16, 16, 4096));

    let report = search.search(repo.path(), "target").unwrap();

    assert_eq!(report.matches().len(), 2);
    assert_eq!(report.matches()[0].line_number(), Some(1));
    assert_eq!(report.matches()[1].line_number(), Some(3));
}

#[test]
fn workspace_search_supports_regex_and_explicit_case_sensitivity() {
    let repo = TempDir::new().unwrap();
    fs::write(
        repo.path().join("symbols.rs"),
        "fn AgentLoop() {}\nfn agent_loop() {}\nconst VALUE: i32 = 1;\n",
    )
    .unwrap();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(16, 16, 4096));

    let exact = search
        .search_with_options(repo.path(), "AgentLoop", false, true)
        .unwrap();
    let regex = search
        .search_with_options(repo.path(), r"fn\s+[a-z_]+", true, true)
        .unwrap();

    assert_eq!(exact.matches().len(), 1);
    assert_eq!(exact.matches()[0].line_number(), Some(1));
    assert_eq!(regex.matches().len(), 1);
    assert_eq!(regex.matches()[0].line_number(), Some(2));
    assert!(
        search
            .search_with_options(repo.path(), "(", true, false)
            .is_err()
    );
}

#[test]
fn workspace_search_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-workspace/src/search.rs",
            include_str!("../src/search.rs"),
            280,
        ),
        (
            "crates/desktoplab-workspace/tests/workspace_search.rs",
            include_str!("workspace_search.rs"),
            170,
        ),
        (
            "crates/desktoplab-workspace/src/search_pattern.rs",
            include_str!("../src/search_pattern.rs"),
            80,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("workspace search files should stay focused");
    }
}

fn search_fixture() -> TempDir {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("node_modules/pkg")).unwrap();
    fs::create_dir_all(repo.path().join("apps/desktop/src/features/productization")).unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(repo.path().join(".git/config"), "composer secret").unwrap();
    fs::write(
        repo.path().join(".env"),
        "OPENAI_API_KEY=sk-secret\ncomposer",
    )
    .unwrap();
    fs::write(repo.path().join("node_modules/pkg/index.js"), "composer").unwrap();
    fs::write(repo.path().join("README.md"), "# DesktopLab\n").unwrap();
    fs::write(
        repo.path().join("package.json"),
        "{\"scripts\":{\"test\":\"vitest\"}}\n",
    )
    .unwrap();
    fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn composer_domain() {}\n",
    )
    .unwrap();
    fs::write(
        repo.path()
            .join("apps/desktop/src/features/productization/AgentComposer.tsx"),
        "export function AgentComposer() { return 'composer'; }\n",
    )
    .unwrap();
    fs::write(repo.path().join("image.bin"), [0, 159, 146, 150]).unwrap();
    repo
}
