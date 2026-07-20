use xtask::{LogicalLineLimitViolation, check_logical_line_limit};

#[test]
fn line_count_guard_counts_non_blank_non_comment_lines() {
    let source = r#"
// comment
pub struct Provider;

impl Provider {
    pub fn id(&self) -> &'static str {
        "provider.openai"
    }
}
"#;

    assert_eq!(check_logical_line_limit("provider.rs", source, 6), Ok(()));
    assert_eq!(
        check_logical_line_limit("provider.rs", source, 5),
        Err(LogicalLineLimitViolation {
            path: "provider.rs".to_string(),
            logical_lines: 6,
            max_lines: 5,
        })
    );
}
