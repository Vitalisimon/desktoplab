use crate::tool_schema_builders::{array_output, boolean_output, object, string_output, tool};
use crate::tool_schema_inputs::{
    command_input, commit_input, delete_path_input, patch_file_input, read_file_input, search_input,
};
use crate::{AgentToolRisk, DesktopLabToolRegistry};

impl Default for DesktopLabToolRegistry {
    fn default() -> Self {
        let mut tools = vec![
            tool(
                "desktoplab.list_files",
                "List workspace files.",
                AgentToolRisk::Low,
                false,
                object(&["path"], &[]),
                array_output("entries"),
            ),
            tool(
                "desktoplab.read_file",
                "Read a workspace file.",
                AgentToolRisk::Low,
                false,
                read_file_input(),
                string_output("text"),
            ),
            tool(
                "desktoplab.search_text",
                "Search text inside the workspace.",
                AgentToolRisk::Low,
                false,
                search_input(),
                array_output("matches"),
            ),
            tool(
                "desktoplab.write_file",
                "Create a new workspace file or intentionally replace an entire file. Provide the complete content requested by the user; do not use this tool for a localized edit to existing content.",
                AgentToolRisk::High,
                true,
                object(&["path", "content"], &["path", "content"]),
                boolean_output("changed"),
            ),
            tool(
                "desktoplab.patch_file",
                "Apply a localized exact-text replacement to an existing workspace file after reading it. The expected text must match current bytes, including whitespace, and unrelated content must be preserved.",
                AgentToolRisk::High,
                true,
                patch_file_input(),
                boolean_output("changed"),
            ),
            tool(
                "desktoplab.create_directory",
                "Create a workspace directory, including missing parents.",
                AgentToolRisk::High,
                true,
                object(&["path"], &["path"]),
                boolean_output("changed"),
            ),
            tool(
                "desktoplab.move_path",
                "Move or rename a workspace file or directory without overwriting an existing destination.",
                AgentToolRisk::High,
                true,
                object(&["source", "destination"], &["source", "destination"]),
                boolean_output("changed"),
            ),
            tool(
                "desktoplab.delete_path",
                "Delete a workspace file or directory. Set recursive true explicitly for a non-empty directory.",
                AgentToolRisk::High,
                true,
                delete_path_input(),
                boolean_output("changed"),
            ),
        ];
        tools.extend(crate::tool_schema_process_catalog::process_tools());
        tools.extend([
            tool(
                "desktoplab.git_status",
                "Inspect Git status to identify all changed and untracked paths. Do not repeat this call while its successful observation is still current.",
                AgentToolRisk::Low,
                false,
                object(&[], &[]),
                array_output("entries"),
            ),
            tool(
                "desktoplab.git_diff",
                "Inspect tracked file changes after status; omit path for the complete tracked diff or provide one workspace-relative path.",
                AgentToolRisk::Low,
                false,
                object(&["path"], &[]),
                string_output("diff"),
            ),
            tool(
                "desktoplab.create_checkpoint",
                "Create a local work checkpoint.",
                AgentToolRisk::Medium,
                false,
                object(&["label"], &["label"]),
                string_output("ref"),
            ),
            tool(
                "desktoplab.run_tests",
                "Run a targeted test command.",
                AgentToolRisk::Medium,
                true,
                command_input(),
                boolean_output("passed"),
            ),
            tool(
                "desktoplab.commit_changes",
                "Commit approved workspace changes.",
                AgentToolRisk::High,
                true,
                commit_input(),
                string_output("status"),
            ),
        ]);
        tools.extend(crate::tool_schema_control_catalog::control_tools());
        tools.push(tool(
            "desktoplab.push_changes",
            "Push approved workspace commits.",
            AgentToolRisk::High,
            true,
            object(&["remote", "branch"], &["remote", "branch"]),
            string_output("status"),
        ));
        Self::from_tools(tools)
    }
}
