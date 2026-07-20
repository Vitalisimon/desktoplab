import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const packagedLaunchSmokes = [
  "scripts/packaging/macos-runtime-ownership-smoke.sh",
  "scripts/packaging/macos-install-smoke.sh",
  "scripts/packaging/linux-appimage-smoke.sh",
  "scripts/packaging/linux-deb-smoke.sh",
  "scripts/packaging/linux-rpm-smoke.sh",
];

test("packaged launch smokes isolate DesktopLab state without replacing the user home", () => {
  for (const path of packagedLaunchSmokes) {
    const source = readFileSync(path, "utf8");
    assert.doesNotMatch(source, /(?:^|\s)HOME=/m, `${path} replaces the runtime and tool home`);
    assert.match(source, /DESKTOPLAB_APP_DATA_DIR=/, `${path} does not isolate DesktopLab state`);
  }
});

test("packaged state-isolation contract stays focused", () => {
  const logicalLines = readFileSync("scripts/packaging/packaged-app-data-isolation.test.mjs", "utf8")
    .split("\n")
    .filter((line) => line.trim()).length;
  assert.ok(logicalLines <= 40, `packaged app-data isolation test has ${logicalLines} logical lines`);
});
