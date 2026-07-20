#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalCommandClass {
    Routine,
    DependencyInstall,
    GeneratedArtifact,
}

#[must_use]
pub fn classify_terminal_command(command: &str) -> TerminalCommandClass {
    let normalized = command
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ");
    if is_dependency_install_command(&normalized) {
        TerminalCommandClass::DependencyInstall
    } else if is_generated_artifact_command(&normalized) {
        TerminalCommandClass::GeneratedArtifact
    } else {
        TerminalCommandClass::Routine
    }
}

fn is_dependency_install_command(command: &str) -> bool {
    command.starts_with("npm install")
        || command.starts_with("npm i")
        || command.starts_with("npm ci")
        || command.starts_with("pnpm install")
        || command.starts_with("pnpm add")
        || command.starts_with("yarn install")
        || command.starts_with("yarn add")
        || command.starts_with("bun install")
        || command.starts_with("bun add")
        || command.starts_with("cargo add")
        || command.starts_with("cargo install")
        || command.starts_with("pip install")
        || command.starts_with("poetry add")
        || command.starts_with("bundle install")
        || command.starts_with("go get")
        || command.starts_with("swift package resolve")
        || command.starts_with("swift package update")
}

fn is_generated_artifact_command(command: &str) -> bool {
    command.starts_with("npm run build")
        || command.starts_with("pnpm build")
        || command.starts_with("pnpm run build")
        || command.starts_with("yarn build")
        || command.starts_with("yarn run build")
        || command.starts_with("bun run build")
        || command.starts_with("cargo build")
        || command.starts_with("go build")
        || command.starts_with("swift build")
        || command.starts_with("next build")
        || command.starts_with("vite build")
        || command == "tsc"
        || command.starts_with("tsc ")
}
