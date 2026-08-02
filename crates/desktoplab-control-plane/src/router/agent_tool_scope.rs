use super::LocalApiRouter;

impl LocalApiRouter {
    pub(super) fn agent_tool_registry_for_model(
        &self,
        model_id: Option<&str>,
    ) -> Result<desktoplab_agent_engine::DesktopLabToolRegistry, String> {
        let registry = self.agent_tool_registry()?;
        Ok(if model_id.is_some_and(model_is_inspection_only) {
            registry.retaining(|tool| inspection_tool_allowed(tool.id()))
        } else {
            registry
        })
    }

    pub(super) fn backend_tool_schemas_for_model(
        &self,
        model_id: Option<&str>,
        suppressed_tool: Option<&str>,
    ) -> Result<Vec<desktoplab_backends::BackendToolSchema>, String> {
        Ok(self
            .agent_tool_registry_for_model(model_id)?
            .tools()
            .iter()
            .filter(|tool| Some(tool.id()) != suppressed_tool)
            .map(|tool| {
                desktoplab_backends::BackendToolSchema::new(
                    tool.id(),
                    tool.description(),
                    tool.input_schema().clone(),
                )
            })
            .collect())
    }

    pub(super) fn agent_tool_ids_for_model(
        &self,
        model_id: Option<&str>,
    ) -> Result<String, String> {
        Ok(self
            .agent_tool_registry_for_model(model_id)?
            .tools()
            .iter()
            .map(|tool| tool.id())
            .collect::<Vec<_>>()
            .join(", "))
    }
}

fn inspection_tool_allowed(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "desktoplab.list_files"
            | "desktoplab.read_file"
            | "desktoplab.search_text"
            | "desktoplab.git_status"
            | "desktoplab.git_diff"
            | "desktoplab.update_plan"
            | "desktoplab.complete"
            | "desktoplab.clarify"
    )
}

fn model_is_inspection_only(model_id: &str) -> bool {
    desktoplab_model_manager::ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .find(|variant| variant.model_id() == model_id)
        .is_some_and(|variant| {
            variant
                .capabilities()
                .iter()
                .any(|capability| capability == "inspection_only")
        })
}
