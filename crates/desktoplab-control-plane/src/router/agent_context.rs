use desktoplab_agent_engine::{
    AgentContext, AgentContextBuilder, AgentContextPlanner, AgentRouteContextCapabilities,
    ContextCandidate, ContextSectionKind,
};
use desktoplab_workspace::{
    HybridRepoRetriever, RepoCodeIndexer, RepoIndexFreshnessGuard, RepoIndexLimits,
    RepositoryInspector,
};
use std::path::Path;

use super::LocalApiRouter;

impl LocalApiRouter {
    pub(super) fn safe_workspace_context(
        &mut self,
        workspace_id: &str,
        prompt: &str,
        context_paths: &[String],
        external_attachments: &[serde_json::Value],
        session_id: Option<&str>,
    ) -> Option<AgentContext> {
        let workspace = self.workspace_record()?;
        if workspace.workspace_id != workspace_id {
            return None;
        }
        let root = Path::new(&workspace.root_path);
        let inspection = RepositoryInspector::new(256).inspect(root).ok()?;
        let index = RepoCodeIndexer::new(RepoIndexLimits::new(2_048, 64 * 1024))
            .build(root)
            .ok()?;
        let freshness = RepoIndexFreshnessGuard::validate(&index, root);
        let retrieval = HybridRepoRetriever::new(&index).retrieve(prompt, 8, &freshness);
        let mut candidates = vec![
            ContextCandidate::new(
                ContextSectionKind::SystemPolicy,
                host_system_policy(),
                "desktoplab.policy",
            ),
            ContextCandidate::new(ContextSectionKind::UserGoal, prompt, "user.goal"),
            ContextCandidate::new(
                ContextSectionKind::ToolSchemas,
                format!("available_tools={}", self.agent_tool_ids().ok()?),
                "desktoplab.tool-schemas",
            ),
            ContextCandidate::new(
                ContextSectionKind::RepositorySummary,
                inspection.summary_text(),
                "repository.inspection",
            ),
        ];
        for (path, contents) in
            crate::workspace_files::selected_context_files(&workspace.root_path, context_paths)
        {
            candidates.push(ContextCandidate::exact_file(path, contents, true));
        }
        for item in retrieval.items() {
            candidates.push(ContextCandidate::new(
                ContextSectionKind::RetrievedEvidence,
                format!(
                    "retrieved:{}:{}-{}\n{}",
                    item.provenance().path(),
                    item.provenance().start_line(),
                    item.provenance().end_line(),
                    item.snippet()
                ),
                format!(
                    "rag:{}:{}",
                    item.provenance().path(),
                    item.provenance().content_hash()
                ),
            ));
        }
        for attachment in external_attachments {
            if let Some((name, contents)) =
                super::agent_attachments::external_attachment_text(attachment)
            {
                candidates.push(ContextCandidate::exact_file(
                    format!("external/{name}"),
                    contents,
                    false,
                ));
            }
        }
        if let Some(memories) = self.workspace_memories.get(workspace_id) {
            for memory in memories {
                candidates.push(ContextCandidate::new(
                    ContextSectionKind::WorkspaceMemory,
                    memory.context_text(),
                    memory.provenance(),
                ));
            }
        }
        if let Some(session_id) = session_id {
            self.refresh_agent_context_compaction(session_id);
        }
        if let Some(transcript) = session_id
            .and_then(|id| self.sessions.get(id))
            .as_ref()
            .and_then(|session| {
                super::agent_transcript::context_transcript_with_compaction(
                    session,
                    session_id.and_then(|id| self.agent_context_compactions.get(id)),
                    24,
                )
            })
        {
            candidates.push(ContextCandidate::new(
                ContextSectionKind::PriorTranscript,
                transcript,
                format!("session-transcript:{}", session_id.unwrap_or_default()),
            ));
        }

        let capabilities = self.context_route_capabilities(freshness.is_fresh());
        let plan = AgentContextPlanner::plan(&capabilities, candidates);
        let mut builder = AgentContextBuilder::new(plan.budget().max_bytes());
        for item in plan
            .items()
            .iter()
            .filter(|item| item.kind() != ContextSectionKind::UserGoal)
        {
            builder = builder.with_workspace_fact(
                format!("context_reason={:?}\n{}", item.reason(), item.text()),
                item.provenance(),
            );
        }
        Some(builder.build())
    }

    pub fn workspace_context_for_prompt_for_test(
        &mut self,
        workspace_id: &str,
        prompt: &str,
        context_paths: &[String],
    ) -> Option<String> {
        self.safe_workspace_context(workspace_id, prompt, context_paths, &[], None)
            .map(|context| context.text().to_string())
    }

    pub fn workspace_context_for_session_prompt_for_test(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        prompt: &str,
    ) -> Option<String> {
        self.safe_workspace_context(workspace_id, prompt, &[], &[], Some(session_id))
            .map(|context| context.text().to_string())
    }

    fn context_route_capabilities(&self, rag_fresh: bool) -> AgentRouteContextCapabilities {
        let model_id =
            crate::execution_routes::local_model_id_from_route_id(&self.selected_route_id)
                .or_else(|| self.readiness.model_id().map(ToString::to_string));
        let context_tokens = model_id
            .as_deref()
            .and_then(|model_id| {
                crate::model_routes::agent_context_window_tokens(
                    model_id,
                    self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
                )
            })
            .map(|tokens| tokens as usize)
            .filter(|tokens| *tokens > 0)
            .unwrap_or(32_768);
        let max_bytes = context_budget_bytes(context_tokens);
        AgentRouteContextCapabilities::new(context_tokens, max_bytes)
            .with_repo_rag(rag_fresh)
            .with_long_context(context_tokens >= 131_072)
    }

    fn refresh_agent_context_compaction(&mut self, session_id: &str) {
        let Some(session) = self.sessions.get(session_id) else {
            return;
        };
        let Some(compaction) = super::agent_compaction::AgentContextCompaction::build(&session)
        else {
            return;
        };
        if self.agent_context_compactions.get(session_id) == Some(&compaction) {
            return;
        }
        self.agent_context_compactions
            .insert(session_id.to_string(), compaction);
        self.persist_agent_context_compactions();
    }
}

fn context_budget_bytes(context_tokens: usize) -> usize {
    const MIN_CONTEXT_BYTES: usize = 64 * 1024;
    const MAX_CONTEXT_BYTES: usize = 2 * 1024 * 1024;
    context_tokens
        .saturating_mul(3)
        .saturating_mul(4)
        .checked_div(4)
        .unwrap_or(MAX_CONTEXT_BYTES)
        .clamp(MIN_CONTEXT_BYTES, MAX_CONTEXT_BYTES)
}

fn host_system_policy() -> String {
    let shell = if cfg!(windows) {
        "powershell.exe"
    } else {
        "/bin/sh"
    };
    let separator = if cfg!(windows) { r"\" } else { "/" };
    format!(
        "DesktopLab owns the session, policy, approvals and repository evidence. \
host_os={}; command_shell={shell}; path_separator={separator}. \
Terminal and test commands must use this host-native shell and path syntax. \
Only the current session transcript provides conversational continuity. Explicitly saved workspace memory never proves current workspace state or that an action still applies; use current repository evidence before making such claims.",
        std::env::consts::OS
    )
}

#[cfg(test)]
mod tests {
    use super::{context_budget_bytes, host_system_policy};

    #[test]
    fn context_budget_scales_past_legacy_sixty_four_kib_cap() {
        assert_eq!(context_budget_bytes(8_192), 64 * 1024);
        assert_eq!(context_budget_bytes(131_072), 393_216);
        assert_eq!(context_budget_bytes(1_000_000), 2 * 1024 * 1024);
    }

    #[test]
    fn host_policy_declares_native_os_and_shell() {
        let policy = host_system_policy();
        assert!(policy.contains(&format!("host_os={}", std::env::consts::OS)));
        if cfg!(windows) {
            assert!(policy.contains("command_shell=powershell.exe"));
        } else {
            assert!(policy.contains("command_shell=/bin/sh"));
        }
        assert!(policy.contains("current session transcript"));
        assert!(policy.contains("never proves current workspace state"));
    }
}
