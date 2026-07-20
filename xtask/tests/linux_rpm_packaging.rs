use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn linux_dev_packaging_uses_owned_rpm_builder_after_tauri_appimage_and_deb() {
    let build_script = read("scripts/packaging/build-dev.sh");
    let build_metadata = read("scripts/packaging/prepare-build-metadata.mjs");
    let tree_state = read("scripts/packaging/git-tree-state.mjs");
    let linux_config = read("apps/desktop/src-tauri/tauri.linux.conf.json");
    let rpm_builder = read("scripts/packaging/linux-rpm-build.sh");

    assert!(linux_config.contains(r#""targets": ["appimage", "deb"]"#));
    assert!(!linux_config.contains(r#""rpm""#));
    assert!(build_script.contains("--bundles appimage,deb"));
    assert!(build_script.contains("scripts/packaging/linux-rpm-build.sh"));
    assert!(!build_script.contains("--bundles appimage,deb,rpm"));
    assert!(build_script.contains("prepare-build-metadata.mjs"));
    assert!(build_metadata.contains(r#"commitSha: git(["rev-parse", "HEAD"])"#));
    assert!(build_metadata.contains(r#"import { gitTreeState } from "./git-tree-state.mjs""#));
    assert!(build_metadata.contains("treeState: gitTreeState(root)"));
    assert!(tree_state.contains(r#"["diff", "--quiet", "HEAD", "--"]"#));
    assert!(tree_state.contains(r#"["ls-files", "--others", "--exclude-standard"]"#));
    assert!(build_metadata.contains(r#"resources: { [metadataPath]: "DesktopLabBuild.json" }"#));
    assert!(rpm_builder.contains("rpmbuild -bb"));
    assert!(rpm_builder.contains("/usr/bin/desktoplab-desktop"));
}

#[test]
fn linux_rpm_packaging_test_stays_small() {
    xtask::check_logical_line_limit(
        "xtask/tests/linux_rpm_packaging.rs",
        include_str!("linux_rpm_packaging.rs"),
        80,
    )
    .expect("linux rpm packaging test should stay focused");
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_path(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn repo_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should have workspace parent")
        .join(path)
}
