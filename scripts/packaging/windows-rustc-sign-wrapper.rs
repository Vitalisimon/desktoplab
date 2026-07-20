use std::collections::BTreeSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("DesktopLab rustc signing wrapper failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<i32, String> {
    let mut arguments = env::args_os().skip(1);
    let rustc = arguments
        .next()
        .ok_or_else(|| "Cargo did not provide the rustc executable".to_owned())?;
    let rustc_arguments: Vec<_> = arguments.collect();
    let status = Command::new(rustc)
        .args(&rustc_arguments)
        .status()
        .map_err(|error| format!("could not start rustc: {error}"))?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }

    let outputs = portable_executable_outputs(&rustc_arguments);
    if outputs.is_empty() {
        return Ok(0);
    }
    let thumbprint = normalized_thumbprint()?;
    for output in outputs {
        if output.is_file() {
            sign_portable_executable(&output, &thumbprint)?;
        }
    }
    Ok(0)
}

fn portable_executable_outputs(arguments: &[OsString]) -> BTreeSet<PathBuf> {
    if let Some(output) = option_value(arguments, "-o") {
        return [PathBuf::from(output)]
            .into_iter()
            .filter(|path| is_portable_executable(path))
            .collect();
    }
    let Some(output_directory) = option_value(arguments, "--out-dir") else {
        return BTreeSet::new();
    };
    let Some(crate_name) = option_value(arguments, "--crate-name") else {
        return BTreeSet::new();
    };
    let suffix = codegen_option(arguments, "extra-filename").unwrap_or_default();
    let crate_types = option_value(arguments, "--crate-type")
        .and_then(OsStr::to_str)
        .unwrap_or("bin");
    let mut outputs = BTreeSet::new();
    for crate_type in crate_types.split(',') {
        let extension = match crate_type {
            "bin" => Some("exe"),
            "proc-macro" | "dylib" | "cdylib" => Some("dll"),
            _ => None,
        };
        if let Some(extension) = extension {
            outputs.insert(PathBuf::from(&output_directory).join(format!(
                "{}{}.{}",
                crate_name.to_string_lossy(),
                suffix.to_string_lossy(),
                extension
            )));
        }
    }
    outputs
}

fn option_value<'a>(arguments: &'a [OsString], option: &str) -> Option<&'a OsStr> {
    for (index, argument) in arguments.iter().enumerate() {
        if argument == option {
            return arguments.get(index + 1).map(OsString::as_os_str);
        }
        if let Some(argument) = argument.to_str() {
            if let Some(value) = argument.strip_prefix(&format!("{option}=")) {
                return Some(OsStr::new(value));
            }
        }
    }
    None
}

fn codegen_option<'a>(arguments: &'a [OsString], option: &str) -> Option<&'a OsStr> {
    arguments.windows(2).find_map(|pair| {
        (pair[0] == "-C")
            .then(|| pair[1].to_str()?.strip_prefix(&format!("{option}=")))
            .flatten()
            .map(OsStr::new)
    })
}

fn normalized_thumbprint() -> Result<String, String> {
    let thumbprint = env::var("WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT")
        .map_err(|_| "WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT is required".to_owned())?
        .replace(' ', "")
        .to_ascii_uppercase();
    if thumbprint.len() != 40 || !thumbprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("WINDOWS_SIGNING_CERTIFICATE_THUMBPRINT is invalid".to_owned());
    }
    Ok(thumbprint)
}

fn sign_portable_executable(path: &Path, thumbprint: &str) -> Result<(), String> {
    for attempt in 0..10 {
        let status = Command::new("signtool.exe")
            .args(["sign", "/fd", "SHA256", "/sha1", thumbprint, "/s", "My"])
            .arg(&path)
            .status()
            .map_err(|error| format!("could not start signtool.exe: {error}"))?;
        if !status.success() {
            if attempt < 9 {
                thread::sleep(Duration::from_millis(100));
                continue;
            }
            return Err(format!(
                "signtool.exe rejected {} after retries",
                path.display()
            ));
        }
        return Ok(());
    }
    unreachable!()
}

fn is_portable_executable(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("exe") || extension.eq_ignore_ascii_case("dll")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn derives_only_the_current_rustc_portable_output() {
        let arguments = [
            OsString::from("--crate-name"),
            OsString::from("serde_derive"),
            OsString::from("--crate-type=proc-macro"),
            OsString::from("--out-dir"),
            OsString::from("target/debug/deps"),
            OsString::from("-C"),
            OsString::from("extra-filename=-abc123"),
        ];
        assert_eq!(
            portable_executable_outputs(&arguments),
            BTreeSet::from([PathBuf::from("target/debug/deps/serde_derive-abc123.dll")])
        );
    }

    #[test]
    fn explicit_non_pe_output_is_not_signed() {
        let arguments = [OsString::from("-o"), OsString::from("library.rlib")];
        assert!(portable_executable_outputs(&arguments).is_empty());
    }

    #[test]
    fn signs_only_windows_portable_executables() {
        assert!(is_portable_executable(Path::new("build.exe")));
        assert!(is_portable_executable(Path::new("macro.DLL")));
        assert!(!is_portable_executable(Path::new("library.rlib")));
    }
}
