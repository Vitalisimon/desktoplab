use crate::repository_open_target::{self, RepositoryOpenTarget};
use crate::LocalApiServer;
use serde::Serialize;
use std::path::Path;
use tauri::{State, Window};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalApiBootstrap {
    pub base_url: String,
    pub auth_token: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTerminalCommandResponse {
    pub terminal_id: String,
    pub workspace_id: String,
    pub state: String,
    pub command: String,
    pub cwd: String,
    pub approval: UserTerminalCommandApproval,
    pub events: Vec<UserTerminalCommandEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTerminalCommandApproval {
    pub approval_id: String,
    pub state: String,
    pub copy: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTerminalCommandEvent {
    pub event_id: String,
    pub kind: String,
    pub stdout: String,
    pub stderr: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout_truncated: bool,
    pub redacted: bool,
}

#[tauri::command]
pub fn local_api_bootstrap(state: State<'_, LocalApiServer>) -> Result<LocalApiBootstrap, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "local api state lock poisoned".to_string())?;
    let api = guard
        .as_ref()
        .ok_or_else(|| "local api is not running".to_string())?;

    Ok(LocalApiBootstrap {
        base_url: api.base_url().to_string(),
        auth_token: api.auth_token().to_string(),
    })
}

#[tauri::command]
pub fn open_repository_in_file_manager(path: String) -> Result<(), String> {
    repository_open_target::open_repository(Path::new(&path), "file_manager")
}

#[tauri::command]
pub fn repository_open_targets() -> Vec<RepositoryOpenTarget> {
    repository_open_target::available_targets()
}

#[tauri::command]
pub fn open_repository_in_target(path: String, target_id: String) -> Result<(), String> {
    repository_open_target::open_repository(Path::new(&path), &target_id)
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    external_url::open(&url)
}

#[tauri::command]
pub fn run_user_terminal_command(
    _workspace_id: String,
    _workspace_path: String,
    _command: String,
    _cwd: Option<String>,
) -> Result<UserTerminalCommandResponse, String> {
    Err("native terminal command is disabled; use the local api terminal route".to_string())
}

#[tauri::command]
pub fn start_window_drag(window: Window) -> Result<(), String> {
    window.start_dragging().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn toggle_window_maximized(window: Window) -> Result<(), String> {
    let is_maximized = window.is_maximized().map_err(|error| error.to_string())?;
    if is_maximized {
        window.unmaximize().map_err(|error| error.to_string())
    } else {
        window.maximize().map_err(|error| error.to_string())
    }
}

mod external_url {
    pub fn open(url: &str) -> Result<(), String> {
        let validated = validate_https_url(url)?;
        let status = opener_command(validated)
            .status()
            .map_err(|error| format!("could not open browser: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("browser open command failed".to_string())
        }
    }

    fn validate_https_url(url: &str) -> Result<&str, String> {
        let trimmed = url.trim();
        if !trimmed.starts_with("https://") {
            return Err("DesktopLab opens only HTTPS consent links.".to_string());
        }
        if trimmed.contains(['\n', '\r', '\0']) || trimmed.contains('"') {
            return Err("Consent link contains unsupported characters.".to_string());
        }
        Ok(trimmed)
    }

    #[cfg(target_os = "macos")]
    fn opener_command(url: &str) -> std::process::Command {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    }

    #[cfg(target_os = "windows")]
    fn opener_command(url: &str) -> std::process::Command {
        let mut command = std::process::Command::new("rundll32");
        command.args(["url.dll,FileProtocolHandler", url]);
        command
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn opener_command(url: &str) -> std::process::Command {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    }

    #[cfg(test)]
    mod tests {
        use super::validate_https_url;

        #[test]
        fn accepts_https_consent_urls() {
            assert_eq!(
                validate_https_url(" https://auth.openai.com/codex/device ").unwrap(),
                "https://auth.openai.com/codex/device"
            );
        }

        #[test]
        fn rejects_non_https_or_shell_sensitive_urls() {
            assert!(validate_https_url("http://localhost:1455/callback").is_err());
            assert!(validate_https_url("https://auth.openai.com/\"\n").is_err());
        }
    }
}
