use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryOpenTarget {
    pub id: String,
    pub label: String,
    pub kind: String,
}

pub fn available_targets() -> Vec<RepositoryOpenTarget> {
    let mut targets = vec![target("file_manager", file_manager_label(), "file_manager")];
    targets.extend(installed_ide_targets());
    targets
}

pub fn open_repository(path: &Path, target_id: &str) -> Result<(), String> {
    if !path.is_dir() {
        return Err("repository path is not a directory".to_string());
    }
    if target_id == "file_manager" {
        return open_file_manager(path);
    }
    let Some(target) = installed_ide_specs()
        .into_iter()
        .find(|target| target.id == target_id && target.installed())
    else {
        return Err("repository open target is unavailable".to_string());
    };
    target.open(path)
}

fn target(id: &str, label: &str, kind: &str) -> RepositoryOpenTarget {
    RepositoryOpenTarget {
        id: id.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
    }
}

fn installed_ide_targets() -> Vec<RepositoryOpenTarget> {
    installed_ide_specs()
        .into_iter()
        .filter(|candidate| candidate.installed())
        .map(|candidate| target(candidate.id, candidate.label, "ide"))
        .collect()
}

struct IdeTarget {
    id: &'static str,
    label: &'static str,
    command: &'static str,
    application_paths: &'static [&'static str],
}

impl IdeTarget {
    fn installed(&self) -> bool {
        let known_path = self
            .application_paths
            .iter()
            .any(|path| Path::new(path).exists());
        known_path || command_available(self.command)
    }

    fn open(&self, path: &Path) -> Result<(), String> {
        open_ide(self.command, path)
    }
}

#[cfg(target_os = "macos")]
fn installed_ide_specs() -> Vec<IdeTarget> {
    vec![
        IdeTarget {
            id: "vscode",
            label: "Visual Studio Code",
            command: "Visual Studio Code",
            application_paths: &["/Applications/Visual Studio Code.app"],
        },
        IdeTarget {
            id: "cursor",
            label: "Cursor",
            command: "Cursor",
            application_paths: &["/Applications/Cursor.app"],
        },
        IdeTarget {
            id: "zed",
            label: "Zed",
            command: "Zed",
            application_paths: &["/Applications/Zed.app"],
        },
        IdeTarget {
            id: "xcode",
            label: "Xcode",
            command: "Xcode",
            application_paths: &["/Applications/Xcode.app"],
        },
        IdeTarget {
            id: "intellij",
            label: "IntelliJ IDEA",
            command: "IntelliJ IDEA",
            application_paths: &["/Applications/IntelliJ IDEA.app"],
        },
        IdeTarget {
            id: "android-studio",
            label: "Android Studio",
            command: "Android Studio",
            application_paths: &["/Applications/Android Studio.app"],
        },
        IdeTarget {
            id: "antigravity",
            label: "Antigravity",
            command: "Antigravity",
            application_paths: &["/Applications/Antigravity.app"],
        },
    ]
}

#[cfg(target_os = "linux")]
fn installed_ide_specs() -> Vec<IdeTarget> {
    vec![
        IdeTarget {
            id: "vscode",
            label: "Visual Studio Code",
            command: "code",
            application_paths: &[],
        },
        IdeTarget {
            id: "cursor",
            label: "Cursor",
            command: "cursor",
            application_paths: &[],
        },
        IdeTarget {
            id: "zed",
            label: "Zed",
            command: "zed",
            application_paths: &[],
        },
    ]
}

#[cfg(target_os = "windows")]
fn installed_ide_specs() -> Vec<IdeTarget> {
    vec![
        IdeTarget {
            id: "vscode",
            label: "Visual Studio Code",
            command: "code",
            application_paths: &[],
        },
        IdeTarget {
            id: "cursor",
            label: "Cursor",
            command: "cursor",
            application_paths: &[],
        },
        IdeTarget {
            id: "zed",
            label: "Zed",
            command: "zed",
            application_paths: &[],
        },
    ]
}

#[cfg(target_os = "macos")]
fn command_available(application: &str) -> bool {
    let Some(home) = std::env::var_os("HOME") else {
        return false;
    };
    Path::new(&home)
        .join("Applications")
        .join(format!("{application}.app"))
        .is_dir()
}

#[cfg(not(target_os = "macos"))]
fn command_available(command: &str) -> bool {
    let probe = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    std::process::Command::new(probe)
        .arg(command)
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "macos")]
fn open_ide(application: &str, path: &Path) -> Result<(), String> {
    run_command(
        "/usr/bin/open",
        &[
            std::ffi::OsStr::new("-a"),
            std::ffi::OsStr::new(application),
            path.as_os_str(),
        ],
    )
}

#[cfg(not(target_os = "macos"))]
fn open_ide(command: &str, path: &Path) -> Result<(), String> {
    run_command(command, &[path.as_os_str()])
}

#[cfg(target_os = "macos")]
fn open_file_manager(path: &Path) -> Result<(), String> {
    run_command("/usr/bin/open", &[path.as_os_str()])
}

#[cfg(target_os = "linux")]
fn open_file_manager(path: &Path) -> Result<(), String> {
    run_command("xdg-open", &[path.as_os_str()])
}

#[cfg(target_os = "windows")]
fn open_file_manager(path: &Path) -> Result<(), String> {
    run_command("explorer", &[path.as_os_str()])
}

fn run_command(command: &str, args: &[&std::ffi::OsStr]) -> Result<(), String> {
    let status = std::process::Command::new(command)
        .args(args)
        .status()
        .map_err(|error| format!("repository open target failed to start: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "repository open target failed".to_string())
}

#[cfg(target_os = "macos")]
fn file_manager_label() -> &'static str {
    "Finder"
}

#[cfg(target_os = "windows")]
fn file_manager_label() -> &'static str {
    "File Explorer"
}

#[cfg(all(unix, not(target_os = "macos")))]
fn file_manager_label() -> &'static str {
    "File manager"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_open_target_source_stays_small() {
        xtask::check_logical_line_limit(
            "apps/desktop/src-tauri/src/repository_open_target.rs",
            include_str!("repository_open_target.rs"),
            280,
        )
        .expect("repository open target source should stay focused");
    }

    #[test]
    fn targets_always_start_with_the_native_file_manager() {
        let targets = available_targets();
        assert_eq!(targets[0].id, "file_manager");
        assert_eq!(targets[0].kind, "file_manager");
        assert!(targets.iter().skip(1).all(|target| target.kind == "ide"));
    }

    #[test]
    fn unknown_targets_fail_before_process_execution() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            open_repository(root.path(), "unknown").unwrap_err(),
            "repository open target is unavailable"
        );
    }
}
