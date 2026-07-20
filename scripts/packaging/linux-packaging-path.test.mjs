import assert from "node:assert/strict";
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("Linux packaging uses active toolchains without inheriting unrelated host bins", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-linux-path-"));
  const toolchain = join(root, "toolchain");
  const unrelated = join(root, "unrelated");

  try {
    for (const directory of [toolchain, unrelated]) {
      mkdirSync(directory, { recursive: true });
    }
    for (const command of ["node", "npm", "cargo", "rustc"]) {
      const path = join(toolchain, command);
      writeFileSync(path, "#!/bin/sh\nexit 0\n");
      chmodSync(path, 0o755);
    }

    const result = spawnSync("/bin/bash", ["scripts/packaging/linux-packaging-path.sh"], {
      cwd: process.cwd(),
      encoding: "utf8",
      env: { ...process.env, PATH: `${toolchain}:${unrelated}:/usr/bin:/bin` },
    });

    assert.equal(result.status, 0, result.stderr);
    const entries = result.stdout.trim().split(":");
    assert.equal(entries[0], toolchain);
    assert.equal(entries.includes(unrelated), false);
    assert.equal(entries.includes("/usr/bin"), true);
    assert.equal(new Set(entries).size, entries.length);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Linux packaging keeps the active Rust toolchain ahead of system Node tools", () => {
  const root = mkdtempSync(join(tmpdir(), "desktoplab-linux-split-path-"));
  const rustToolchain = join(root, "rust-toolchain");
  const systemTools = join(root, "system-tools");

  try {
    for (const directory of [rustToolchain, systemTools]) {
      mkdirSync(directory, { recursive: true });
    }
    for (const command of ["cargo", "rustc"]) {
      const path = join(rustToolchain, command);
      writeFileSync(path, "#!/bin/sh\nexit 0\n");
      chmodSync(path, 0o755);
    }
    for (const command of ["node", "npm"]) {
      const path = join(systemTools, command);
      writeFileSync(path, "#!/bin/sh\nexit 0\n");
      chmodSync(path, 0o755);
    }

    const result = spawnSync("/bin/bash", ["scripts/packaging/linux-packaging-path.sh"], {
      cwd: process.cwd(),
      encoding: "utf8",
      env: { ...process.env, PATH: `${rustToolchain}:${systemTools}:/usr/bin:/bin` },
    });

    assert.equal(result.status, 0, result.stderr);
    const entries = result.stdout.trim().split(":");
    assert.equal(entries[0], rustToolchain);
    assert.equal(entries[1], systemTools);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
