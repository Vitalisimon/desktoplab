use std::io::{Error, Result};
use std::path::Path;
use std::process::{Command, Output};

pub(crate) fn restrict_to_current_user(path: &Path) -> Result<()> {
    let sid = current_user_sid()?;
    run_icacls(path, &["/inheritance:r"])?;
    run_icacls(path, &["/grant:r", &format!("*{sid}:(F)")])?;
    Ok(())
}

fn current_user_sid() -> Result<String> {
    let output = Command::new("whoami.exe")
        .args(["/user", "/fo", "csv", "/nh"])
        .output()?;
    require_success("windows_sid_lookup_failed", &output)?;
    let text = String::from_utf8_lossy(&output.stdout);
    let start = text
        .find("S-1-")
        .ok_or_else(|| Error::other("windows_sid_missing"))?;
    let tail = &text[start..];
    let end = tail
        .find(|character: char| character != 'S' && character != '-' && !character.is_ascii_digit())
        .unwrap_or(tail.len());
    Ok(tail[..end].to_string())
}

fn run_icacls(path: &Path, arguments: &[&str]) -> Result<()> {
    let output = Command::new("icacls.exe")
        .arg(path)
        .args(arguments)
        .output()?;
    require_success("windows_acl_restriction_failed", &output)
}

fn require_success(reason: &str, output: &Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    Err(Error::other(format!("{reason}:{}", detail.trim())))
}
