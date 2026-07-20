#![forbid(unsafe_code)]

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("product-truth-real") => run_product_truth_real(),
        _ => {
            eprintln!("usage: cargo run -p xtask -- product-truth-real");
            ExitCode::from(2)
        }
    }
}

fn run_product_truth_real() -> ExitCode {
    match Command::new("npm")
        .args(["run", "product:truth:real"])
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("failed to run product truth regression: {error}");
            ExitCode::from(1)
        }
    }
}
