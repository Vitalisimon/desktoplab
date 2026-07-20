import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export const reliabilityProfilesBySeed = Object.freeze({
  7: "medium",
  19: "large_context",
  43: "long_session",
  71: "restart_resume",
  97: "deny_cancel_recovery",
});

const profiles = Object.freeze({
  medium: profile("medium", 512, 2_048),
  large_context: profile("large_context", 2_048, 8_192, {
    preludePrompts: ["Search docs/generated for RELEASE_CONTEXT_SENTINEL_2047, report its file path and value, and do not modify anything."],
  }),
  long_session: profile("long_session", 768, 4_096, {
    preludePrompts: [
      "Inspect the repository structure and identify the fixture implementation and test files. Do not modify anything.",
      "Read release-note.md and state its current candidate value. Do not modify anything.",
      "Show Git status and confirm whether the workspace is clean. Do not modify anything.",
    ],
  }),
  restart_resume: profile("restart_resume", 768, 4_096, {
    preludePrompts: ["Inspect the repository and remember the fixture test command. Do not modify anything."],
    restartAfterPrelude: true,
  }),
  deny_cancel_recovery: profile("deny_cancel_recovery", 1_024, 8_192, {
    denyFirstApproval: true,
    cancelFirstReadOnly: true,
    memoryPressureMb: 768,
  }),
});

export function reliabilityProfile(profileId) {
  const value = profiles[profileId];
  if (!value) throw new Error(`unknown reliability profile ${profileId}`);
  return value;
}

export function profileForSeed(seed) {
  return reliabilityProfilesBySeed[seed] ?? "medium";
}

export function prepareProfileFiles(root, profileId) {
  const selected = reliabilityProfile(profileId);
  const directory = join(root, "docs/generated");
  mkdirSync(directory, { recursive: true });
  for (let index = 0; index < selected.fileCount; index += 1) {
    const id = String(index).padStart(4, "0");
    const marker = index === selected.fileCount - 1 ? `RELEASE_CONTEXT_SENTINEL_${id}=verified\n` : "";
    const header = `# Generated reliability document ${id}\n\n${marker}`;
    const filler = `profile=${profileId}; record=${id}; DesktopLab reliability context.\n`;
    const repeats = Math.max(1, Math.ceil((selected.bytesPerFile - header.length) / filler.length));
    writeFileSync(join(directory, `context-${id}.md`), `${header}${filler.repeat(repeats)}`.slice(0, selected.bytesPerFile));
  }
  return selected;
}

function profile(id, fileCount, bytesPerFile, overrides = {}) {
  return Object.freeze({
    id,
    fileCount,
    bytesPerFile,
    preludePrompts: [],
    restartAfterPrelude: false,
    denyFirstApproval: false,
    cancelFirstReadOnly: false,
    memoryPressureMb: 0,
    ...overrides,
  });
}
