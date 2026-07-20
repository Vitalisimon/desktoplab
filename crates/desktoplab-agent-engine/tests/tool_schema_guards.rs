use xtask::check_logical_line_limit;

#[test]
fn tool_schema_registry_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-agent-engine/src/tool_schema.rs",
            include_str!("../src/tool_schema.rs"),
            180,
        ),
        (
            "crates/desktoplab-agent-engine/src/tool_schema_catalog.rs",
            include_str!("../src/tool_schema_catalog.rs"),
            190,
        ),
        (
            "crates/desktoplab-agent-engine/src/tool_schema_builders.rs",
            include_str!("../src/tool_schema_builders.rs"),
            90,
        ),
        (
            "crates/desktoplab-agent-engine/src/tool_schema_control_catalog.rs",
            include_str!("../src/tool_schema_control_catalog.rs"),
            100,
        ),
        (
            "crates/desktoplab-agent-engine/src/tool_schema_inputs.rs",
            include_str!("../src/tool_schema_inputs.rs"),
            160,
        ),
        (
            "crates/desktoplab-agent-engine/src/tool_schema_process_catalog.rs",
            include_str!("../src/tool_schema_process_catalog.rs"),
            120,
        ),
        (
            "crates/desktoplab-agent-engine/src/tool_schema_extensions.rs",
            include_str!("../src/tool_schema_extensions.rs"),
            100,
        ),
        (
            "crates/desktoplab-agent-engine/src/mcp_schema_validator.rs",
            include_str!("../src/mcp_schema_validator.rs"),
            120,
        ),
        (
            "crates/desktoplab-agent-engine/tests/tool_schema_registry.rs",
            include_str!("tool_schema_registry.rs"),
            170,
        ),
        (
            "crates/desktoplab-agent-engine/tests/tool_schema_provider_contract.rs",
            include_str!("tool_schema_provider_contract.rs"),
            140,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("tool schema registry files should stay focused");
    }
}
