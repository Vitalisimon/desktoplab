use desktoplab_tool_gateway::ToolIntent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiFileRefactorFile {
    path: String,
    expected: String,
    replacement: String,
}

impl MultiFileRefactorFile {
    #[must_use]
    pub fn new(
        path: impl Into<String>,
        expected: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            expected: expected.into(),
            replacement: replacement.into(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn expected(&self) -> &str {
        &self.expected
    }

    #[must_use]
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiFileRefactorRequest {
    objective: String,
    files: Vec<MultiFileRefactorFile>,
    validation_command: String,
}

impl MultiFileRefactorRequest {
    #[must_use]
    pub fn new(
        objective: impl Into<String>,
        files: Vec<MultiFileRefactorFile>,
        validation_command: impl Into<String>,
    ) -> Self {
        Self {
            objective: objective.into(),
            files,
            validation_command: validation_command.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiFileRefactorPlan {
    objective: String,
    files: Vec<MultiFileRefactorFile>,
    validation_command: String,
}

impl MultiFileRefactorPlan {
    pub const MAX_FILES: usize = 8;

    pub fn from_request(request: MultiFileRefactorRequest) -> Result<Self, RefactorPlanError> {
        if request.files.len() < 2 {
            return Err(RefactorPlanError::TooFewFiles);
        }
        if request.files.len() > Self::MAX_FILES {
            return Err(RefactorPlanError::TooManyFiles);
        }
        if request.validation_command.trim().is_empty() {
            return Err(RefactorPlanError::MissingValidation);
        }
        if request.files.iter().any(|file| file.path.trim().is_empty()) {
            return Err(RefactorPlanError::MissingPath);
        }
        Ok(Self {
            objective: request.objective,
            files: request.files,
            validation_command: request.validation_command,
        })
    }

    #[must_use]
    pub fn objective(&self) -> &str {
        &self.objective
    }

    #[must_use]
    pub fn files(&self) -> &[MultiFileRefactorFile] {
        &self.files
    }

    #[must_use]
    pub fn validation_command(&self) -> &str {
        &self.validation_command
    }

    #[must_use]
    pub fn checkpoint_label(&self) -> String {
        format!("multi-file-refactor:{}-files", self.files.len())
    }

    #[must_use]
    pub fn planned_tools(&self) -> Vec<ToolIntent> {
        let mut tools = Vec::with_capacity(self.files.len() + 2);
        tools.push(ToolIntent::create_checkpoint(self.checkpoint_label()));
        tools.extend(
            self.files
                .iter()
                .map(|file| ToolIntent::filesystem_write(file.path.clone())),
        );
        tools.push(ToolIntent::test_run(
            self.validation_command.clone(),
            "validate multi-file refactor",
        ));
        tools
    }

    #[must_use]
    pub fn patch_summaries(&self) -> Vec<String> {
        self.files
            .iter()
            .map(|file| {
                format!(
                    "{} expected_bytes={} replacement_bytes={}",
                    file.path(),
                    file.expected().len(),
                    file.replacement().len()
                )
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefactorPlanError {
    TooFewFiles,
    TooManyFiles,
    MissingPath,
    MissingValidation,
}

impl RefactorPlanError {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TooFewFiles => "refactor_requires_multiple_files",
            Self::TooManyFiles => "refactor_patch_set_too_large",
            Self::MissingPath => "refactor_file_path_required",
            Self::MissingValidation => "refactor_validation_required",
        }
    }
}
