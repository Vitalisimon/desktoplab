use std::collections::BTreeMap;
use std::process::{Child, Command};

#[cfg(windows)]
use std::process::Stdio;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub(crate) fn safe_inherited_env() -> BTreeMap<String, String> {
    std::env::vars()
        .filter(|(key, _)| safe_env_key(key))
        .collect()
}

fn safe_env_key(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    matches!(
        key.as_str(),
        "PATH"
            | "PATHEXT"
            | "SYSTEMROOT"
            | "WINDIR"
            | "COMSPEC"
            | "HOME"
            | "USER"
            | "LOGNAME"
            | "USERPROFILE"
            | "LOCALAPPDATA"
            | "APPDATA"
            | "PROGRAMFILES"
            | "PROGRAMFILES(X86)"
            | "PROGRAMW6432"
            | "TEMP"
            | "TMP"
            | "TMPDIR"
            | "SHELL"
            | "LANG"
            | "LC_ALL"
            | "TERM"
            | "CARGO_HOME"
            | "RUSTUP_HOME"
            | "GOPATH"
            | "GOROOT"
            | "JAVA_HOME"
            | "PYTHONHOME"
            | "PYTHONPATH"
            | "VIRTUAL_ENV"
            | "NODE_PATH"
    ) || key.starts_with("LC_")
        || key.starts_with("XDG_")
}

#[cfg(not(windows))]
pub(crate) fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("/bin/sh");
    shell.arg("-c").arg(command);
    shell.process_group(0);
    shell
}

#[cfg(windows)]
pub(crate) fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("powershell.exe");
    shell
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
        ])
        .arg(format!(
            "$utf8 = [System.Text.UTF8Encoding]::new($false); $OutputEncoding = $utf8; [Console]::OutputEncoding = $utf8; {command}"
        ));
    shell
}

#[cfg(unix)]
pub(crate) fn terminate_process_tree(child: &mut Child) -> std::io::Result<()> {
    let _ = Command::new("kill")
        .args(process_group_kill_args(child.id()))
        .status();
    child.kill()
}

#[cfg(unix)]
fn process_group_kill_args(process_id: u32) -> [String; 3] {
    [
        "-KILL".to_string(),
        "--".to_string(),
        format!("-{process_id}"),
    ]
}

#[cfg(windows)]
pub(crate) fn terminate_process_tree(child: &mut Child) -> std::io::Result<()> {
    let _ = Command::new("taskkill.exe")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    child.kill()
}

#[cfg(all(test, unix))]
mod tests {
    use super::process_group_kill_args;

    #[test]
    fn negative_process_group_operand_follows_option_terminator() {
        assert_eq!(
            process_group_kill_args(42),
            ["-KILL", "--", "-42"].map(str::to_string)
        );
    }
}
