#![forbid(unsafe_code)]

pub mod packaging;
pub mod packaging_manifest;
pub mod product_truth;
pub mod test_http;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalLineLimitViolation {
    pub path: String,
    pub logical_lines: usize,
    pub max_lines: usize,
}

pub fn check_logical_line_limit(
    path: impl Into<String>,
    source: &str,
    max_lines: usize,
) -> Result<(), LogicalLineLimitViolation> {
    let logical_lines = count_logical_lines(source);

    if logical_lines <= max_lines {
        return Ok(());
    }

    Err(LogicalLineLimitViolation {
        path: path.into(),
        logical_lines,
        max_lines,
    })
}

#[must_use]
pub fn count_logical_lines(source: &str) -> usize {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}
