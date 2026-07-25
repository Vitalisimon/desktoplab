import assert from "node:assert/strict";
import test from "node:test";

import {
  directSourceAuditRequired,
  isForbiddenPublicTrackedPath,
  PUBLIC_AUDIT_GIT_MAX_BUFFER,
} from "./public-export-audit-policy.mjs";

test("canonical public CI enforces direct source safety automatically", () => {
  assert.equal(directSourceAuditRequired({ args: [], repository: "Vitalisimon/desktoplab" }), true);
  assert.equal(directSourceAuditRequired({ args: [], repository: "Vitalisimon/desktoplab-private-history" }), false);
  assert.equal(directSourceAuditRequired({ args: ["--direct-source"], repository: "" }), true);
});

test("public tracked-path policy rejects transport and private source", () => {
  for (const path of ["AGENTS.md", "PUBLIC_EXPORT_MANIFEST.json", "docs/private.md", ".env.local"]) {
    assert.equal(isForbiddenPublicTrackedPath(path), true, path);
  }
  for (const path of ["Cargo.lock", "src/main.rs", "README.md"]) {
    assert.equal(isForbiddenPublicTrackedPath(path), false, path);
  }
});

test("public history audit reserves enough output for a growing repository", () => {
  assert.ok(PUBLIC_AUDIT_GIT_MAX_BUFFER >= 8 * 1024 * 1024);
});
