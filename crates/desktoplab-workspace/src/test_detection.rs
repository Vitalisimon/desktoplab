use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestCommandConfidence {
    High,
    Low,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectedTestCommand {
    command: String,
    confidence: TestCommandConfidence,
    source_path: PathBuf,
}

impl DetectedTestCommand {
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    #[must_use]
    pub fn confidence(&self) -> TestCommandConfidence {
        self.confidence
    }

    #[must_use]
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TestCommandSet {
    commands: Vec<DetectedTestCommand>,
    executed_any_command: bool,
}

impl TestCommandSet {
    #[must_use]
    pub fn has_high_confidence(&self, command: &str) -> bool {
        self.has(command, TestCommandConfidence::High)
    }

    #[must_use]
    pub fn has_low_confidence(&self, command: &str) -> bool {
        self.has(command, TestCommandConfidence::Low)
    }

    #[must_use]
    pub fn requires_confirmation(&self, command: &str) -> bool {
        self.has_low_confidence(command)
    }

    #[must_use]
    pub fn executed_any_command(&self) -> bool {
        self.executed_any_command
    }

    #[must_use]
    pub fn commands(&self) -> &[DetectedTestCommand] {
        &self.commands
    }

    fn push(&mut self, command: &str, confidence: TestCommandConfidence, source_path: &Path) {
        self.commands.push(DetectedTestCommand {
            command: command.to_string(),
            confidence,
            source_path: source_path.to_path_buf(),
        });
    }

    fn has(&self, command: &str, confidence: TestCommandConfidence) -> bool {
        self.commands
            .iter()
            .any(|found| found.command == command && found.confidence == confidence)
    }
}

pub struct TestCommandDetector;

impl TestCommandDetector {
    pub fn detect(root: &Path) -> io::Result<TestCommandSet> {
        let mut set = TestCommandSet::default();

        let package_json = root.join("package.json");
        if package_json.exists() {
            let contents = fs::read_to_string(&package_json)?;
            if contents.contains("\"test\"") {
                set.push("npm test", TestCommandConfidence::High, &package_json);
            }
        }

        let cargo = root.join("Cargo.toml");
        if cargo.exists() {
            set.push("cargo test", TestCommandConfidence::High, &cargo);
        }

        let pyproject = root.join("pyproject.toml");
        if pyproject.exists() {
            set.push("pytest", TestCommandConfidence::Low, &pyproject);
        }

        let go_mod = root.join("go.mod");
        if go_mod.exists() {
            set.push("go test ./...", TestCommandConfidence::High, &go_mod);
        }

        let package_swift = root.join("Package.swift");
        if package_swift.exists() {
            set.push("swift test", TestCommandConfidence::High, &package_swift);
        }

        Ok(set)
    }
}
