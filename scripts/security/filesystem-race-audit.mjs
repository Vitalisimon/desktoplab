import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(fileURLToPath(new URL("../../", import.meta.url)));

export const surfaces = [
  surface("read", "crates/desktoplab-tool-gateway/src/filesystem.rs", "WorkspaceRoot::open", "root.read_text", "capability_open", "platform"),
  surface("write", "crates/desktoplab-tool-gateway/src/filesystem.rs", "WorkspaceRoot::open", "root.write_text", "capability_open", "executor"),
  surface("patch", "crates/desktoplab-tool-gateway/src/patch.rs", "WorkspaceRoot::open", "root.open_update", "stable_open_handle", "executor"),
  surface("multi_file_patch", "crates/desktoplab-control-plane/src/router/agent_sessions.rs", "FilesystemBatchPatchExecutor", "BatchPatchOutcome", "capability_executor", "executor"),
  surface("terminal", "crates/desktoplab-tool-gateway/src/terminal.rs", "contained_existing_path", "TerminalProcessAdapter", "approved_shell_not_os_sandboxed", "policy"),
  surface("git", "crates/desktoplab-workspace/src/product_git.rs", "current_dir(root)", "Command::new(\"git\")", "approved_repository_process", "policy"),
];

function surface(id, file, validationMarker, useMarker, finding, owner) {
  return { id, file, validationMarker, useMarker, finding, owner };
}

export function auditFilesystemSurfaces(root = repoRoot) {
  return surfaces.map((entry) => {
    const source = readFileSync(resolve(root, entry.file), "utf8");
    return {
      ...entry,
      validationObserved: source.includes(entry.validationMarker),
      useObserved: source.includes(entry.useMarker),
      audited: source.includes(entry.validationMarker) && source.includes(entry.useMarker),
    };
  });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const records = auditFilesystemSurfaces();
  process.stdout.write(`${JSON.stringify({ schemaVersion: 1, records }, null, 2)}\n`);
  process.exitCode = records.every((record) => record.audited) ? 0 : 1;
}
