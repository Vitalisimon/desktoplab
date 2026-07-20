import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/continuous-integration.yml", "utf8");

test("continuous integration runs for trusted main changes and manual recovery", () => {
  assert.match(workflow, /on:\n  push:\n    branches:\n      - main/);
  assert.match(workflow, /pull_request:\n    branches:\n      - main/);
  assert.match(workflow, /workflow_dispatch:/);
  assert.equal(
    [...workflow.matchAll(/if: github\.event_name != 'pull_request' \|\| github\.event\.pull_request\.head\.repo\.full_name == github\.repository/g)].length,
    3,
  );
});

test("continuous integration covers contracts, frontend, and the Rust workspace", () => {
  assert.match(workflow, /contracts:\n    name: Contracts and public boundary/);
  assert.match(workflow, /frontend:\n    name: Frontend/);
  assert.match(workflow, /rust:\n    name: Rust workspace/);
  assert.match(workflow, /npm run product:public-export:audit/);
  assert.match(workflow, /npm run desktop:check/);
  assert.match(
    workflow,
    /cargo" metadata --locked --manifest-path apps\/desktop\/src-tauri\/Cargo\.toml/,
  );
  assert.match(workflow, /cargo" test --locked --workspace/);
});

test("continuous integration is read-only and cannot invoke release signing", () => {
  assert.match(workflow, /permissions:\n  contents: read/);
  assert.doesNotMatch(workflow, /contents: write|id-token: write|SIGNPATH|APPLE_KEYCHAIN|PRIVATE_KEY/);
  assert.doesNotMatch(workflow, /uses: [^\n]+@(?![a-f0-9]{40}\b)/);
  assert.match(workflow, /actions\/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 # v7\.0\.0/);
  assert.match(workflow, /actions\/setup-node@820762786026740c76f36085b0efc47a31fe5020 # v7\.0\.0/);
  assert.equal(
    [...workflow.matchAll(/actions\/setup-node@820762786026740c76f36085b0efc47a31fe5020/g)].length,
    3,
  );
});

test("continuous integration is pinned to the isolated self-hosted Linux runner pool", () => {
  assert.equal(
    [...workflow.matchAll(/runs-on:\n      - self-hosted\n      - desktoplab-linux-x64/g)].length,
    3,
  );
  assert.doesNotMatch(workflow, /runs-on: ubuntu-|runs-on: ubuntu-latest/);
  assert.match(workflow, /persist-credentials: false/);
  assert.match(workflow, /"\$HOME\/\.cargo\/bin\/cargo" test --locked --workspace/);
  assert.match(workflow, /cargo" test --locked --workspace --no-fail-fast/);
});
