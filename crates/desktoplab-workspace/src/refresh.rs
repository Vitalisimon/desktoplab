#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRefreshScheduler {
    max_workspaces: usize,
}

impl ContextRefreshScheduler {
    #[must_use]
    pub fn new(max_workspaces: usize) -> Self {
        Self { max_workspaces }
    }

    #[must_use]
    pub fn refresh<I, W>(&self, workspaces: I, succeeds: bool) -> ContextRefreshReport
    where
        I: IntoIterator<Item = W>,
        W: AsRef<str>,
    {
        let observed = workspaces.into_iter().count();
        let refreshed = observed.min(self.max_workspaces);
        ContextRefreshReport {
            refreshed,
            bounded: observed > self.max_workspaces,
            diagnostics: if succeeds {
                "ok".to_string()
            } else {
                "refresh_failed".to_string()
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextRefreshReport {
    refreshed: usize,
    bounded: bool,
    diagnostics: String,
}

impl ContextRefreshReport {
    #[must_use]
    pub fn refreshed_count(&self) -> usize {
        self.refreshed
    }

    #[must_use]
    pub fn is_bounded(&self) -> bool {
        self.bounded
    }

    #[must_use]
    pub fn diagnostics(&self) -> &str {
        &self.diagnostics
    }
}
